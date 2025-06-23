use crate::{config::Config, types::VastInstance, vast::VastClient};
use anyhow::{Context, Result};
use axum::http::StatusCode;
use log::{debug, error, info, warn};
use std::collections::{HashMap, HashSet};
use tokio::{
    sync::{mpsc, oneshot},
    time::{Duration, Instant, interval},
};

#[derive(Clone)]
pub struct InstanceControllerClient {
    sender: mpsc::Sender<InstanceControllerCommand>,
}

impl InstanceControllerClient {
    pub async fn new(config: Config) -> Result<Self> {
        let vast_client = VastClient::new(config.clone());

        let (sender, receiver) = mpsc::channel(100);
        let controller = InstanceController::initialize(vast_client, config.clone(), receiver)
            .await
            .context("Initialize InstanceController")?;

        let sender_clone = sender.clone();
        tokio::task::spawn(async move { controller.background_event_loop(sender_clone).await });

        Ok(Self { sender })
    }

    pub async fn drop(&self, offer_id: u64) -> Result<Result<String, StatusCode>> {
        let (resp_sender, receiver) = oneshot::channel();
        let command = InstanceControllerCommand::Drop {
            offer_id,
            resp_sender,
        };
        self.sender.send(command).await?;

        let resp = receiver.await?;

        Ok(resp)
    }

    pub async fn instances(&self) -> Result<Vec<VastInstance>> {
        let (resp_sender, receiver) = oneshot::channel();
        let command = InstanceControllerCommand::GetAll { resp_sender };
        self.sender.send(command).await?;

        let instances = receiver
            .await?
            .iter()
            .map(|(_, instance)| instance)
            .cloned()
            .collect();

        Ok(instances)
    }

    pub async fn verify(&self, offer_id: u64) -> Result<()> {
        let command = InstanceControllerCommand::VerifyInstance { offer_id };
        self.sender.send(command).await?;
        Ok(())
    }
}

pub struct InstanceController {
    // mapping instance_id -> instance
    instances: HashMap<u64, VastInstance>,
    vast_client: VastClient,
    receiver: mpsc::Receiver<InstanceControllerCommand>,
    config: Config,
}

impl InstanceController {
    pub async fn initialize(
        vast_client: VastClient,
        config: Config,
        receiver: mpsc::Receiver<InstanceControllerCommand>,
    ) -> Result<Self> {
        // create initial instances
        let desired_instances = config.number_instances;
        info!("Creating initial {desired_instances} instances.  Please wait...");
        let start = Instant::now();
        let instances = vast_client
            .create_initial_instances(desired_instances)
            .await
            .context("Initial instance creation")?;
        let instances = instances.into_iter().collect();

        let elapsed = start.elapsed().as_secs_f32();
        info!(
            "Created initial {desired_instances} instances in {:.2} seconds",
            elapsed
        );

        Ok(Self {
            instances,
            vast_client,
            receiver,
            config,
        })
    }

    async fn background_event_loop(
        mut self,
        sender: mpsc::Sender<InstanceControllerCommand>,
    ) -> Result<()> {
        // runs a cleanup task every 30 seconds
        tokio::spawn(async move {
            let mut interval =
                interval(Duration::from_secs(self.config.task_polling_interval_secs));

            loop {
                interval.tick().await;

                let command = InstanceControllerCommand::HandleUnfinishedBusiness;
                if let Err(_) = sender.send(command).await {
                    error!("Instance controller exited.");
                    break;
                }
            }
        });

        // handles all tasks and holds state
        while let Some(command) = self.receiver.recv().await {
            match command {
                InstanceControllerCommand::HandleUnfinishedBusiness => {
                    self.correct_active_instance_count().await;

                    self.check_contemplant_verification().await;

                    let mut instances_dropped = Vec::new();

                    let instances_clone = self.instances.clone();
                    for (instance_id, instance) in instances_clone {
                        // if we shouldn't drop this instance, skip
                        if !instance.should_drop {
                            continue;
                        }

                        match self.vast_client.drop_instance(instance_id.clone()).await {
                            Ok(_) => {
                                info!("Dropped {instance}");
                                instances_dropped.push(instance_id);
                            }
                            Err(e) => {
                                warn!(
                                    "Error on attempt to drop {instance}.  Will try again later. {e}"
                                );
                            }
                        }
                    }

                    self.instances
                        .retain(|instance_id, _| !instances_dropped.contains(&instance_id));

                    self.ensure_sufficient_instances().await;
                }
                InstanceControllerCommand::Drop {
                    offer_id,
                    resp_sender,
                } => {
                    let mut target_instance: Option<u64> = None;
                    // find the instance based on offer_id
                    for (instance_id, instance) in self.instances.iter_mut() {
                        if instance.offer.id == offer_id {
                            instance.should_drop = true;
                            target_instance = Some(instance_id.clone());
                            break;
                        }
                    }

                    let resp = match target_instance {
                        Some(instance_id) => {
                            debug!("Marking {instance_id} to be dropped");
                            Ok(format!("{instance_id} will be dropped"))
                        }
                        None => {
                            warn!(
                                "Attempted to drop offer_id {offer_id} but it isn't known to this magister.  Skipping request."
                            );
                            Err(StatusCode::BAD_REQUEST)
                        }
                    };

                    if let Err(_) = resp_sender.send(resp) {
                        error!("Drop response receiver out of scope.  Exiting");
                        break;
                    }
                }
                InstanceControllerCommand::GetAll { resp_sender } => {
                    if let Err(_) = resp_sender.send(self.instances.clone()) {
                        error!("Get all instances response receiver dropped.  Exiting");
                        break;
                    }
                }
                InstanceControllerCommand::VerifyInstance { offer_id } => {
                    for (_, instance) in self.instances.iter_mut() {
                        if instance.offer.id == offer_id {
                            debug!("Instance {instance} with offer_id {offer_id} verified!");
                            instance.contemplant_verified = true;
                            break;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    async fn check_contemplant_verification(&mut self) {
        // If we haven't heard the initialization ping from the contemplant within
        // <contemplant_verification_timeout_secs>, drop the instance
        for (instance_id, instance) in self.instances.iter_mut() {
            // if it's not verified
            if !instance.contemplant_verified {
                // and it's been longer than contemplant_verification_timeout_secs
                let time_since_creation = instance.creation_time.elapsed();
                if time_since_creation
                    > Duration::from_secs(self.config.contemplant_verification_timeout_secs)
                {
                    warn!(
                        "{instance} with id {instance_id} was created {:.2} seconds ago but hasn't yet been verified.  Dropping.",
                        time_since_creation.as_secs_f32()
                    );
                    instance.should_drop = true;
                }
            }
        }
    }

    // compare our instances to the instances Vast is aware of
    async fn correct_active_instance_count(&mut self) {
        let returned_instance_ids: HashSet<u64> = match self.vast_client.get_instances().await {
            Ok(x) => x.into_iter().collect(),
            Err(e) => {
                warn!(
                    "Error sending command to get updated instance count: {e}.  Will try again later."
                );
                return;
            }
        };

        let mut zombie_instances = Vec::new();
        // This could return instances that are running that aren't for Magister.  Vast doesn't let
        // us query by label, so we can only use this to remove instance ids that we have running
        // but aren't returned by the above api call
        for (instance_id, instance) in self.instances.clone() {
            // We have an instance that vast isn't aware of.  This means the instance was removed
            // via the vast Frontend, and we should remove this from our state.  It doesn't need to
            // be dropped because it already doesn't exist in vast
            if let None = returned_instance_ids.get(&instance_id) {
                info!(
                    "Instance id {instance_id} {instance} was dropped by somone via the Vast.ai frontend.  Removing it from Magister state."
                );
                zombie_instances.push(instance_id);
            }
        }

        // only retain instances that aren't in the list of zombie_instances
        self.instances
            .retain(|instance_id, _| !zombie_instances.contains(&instance_id));
    }
    // requests new instances if we're below config.number_instances
    async fn ensure_sufficient_instances(&mut self) {
        if self.instances.len() < self.config.number_instances {
            let required_instances = self.config.number_instances - self.instances.len();
            info!(
                "Currently at {} / {} instances.  Requesting more...",
                self.instances.len(),
                self.config.number_instances
            );

            let offers = match self.vast_client.find_offers().await {
                Ok(offers) => offers,
                Err(e) => {
                    warn!(
                        "Error finding offers to request new instances.  Will try again later\n{e}"
                    );
                    return;
                }
            };

            let mut new_instances = Vec::new();
            for offer in offers {
                let offer_id = offer.id;
                match self.vast_client.request_new_instance(offer_id).await {
                    Ok(Some(instance_id)) => {
                        let new_instance = VastInstance::new(instance_id, offer);
                        info!("Accepted offer {offer_id} for {new_instance}");
                        new_instances.push((instance_id, new_instance));
                    }
                    Ok(None) => {
                        warn!("Reached Vast rate limit.  Will try to request more instances later");
                        break;
                    }
                    Err(e) => {
                        warn!(
                            "Unable to request offer {offer_id} of a {} in {} with machine_id {} and host_id {} for ${:.2}/hour.\nError: {e}",
                            offer.gpu_name,
                            offer.geolocation,
                            offer.machine_id,
                            offer.host_id,
                            offer.dph_total
                        );
                    }
                }

                if new_instances.len() == required_instances {
                    break;
                }
            }

            for (new_instance_id, new_instance) in new_instances {
                if let Some(old_instance) =
                    self.instances.insert(new_instance_id, new_instance.clone())
                {
                    warn!(
                        "Instance id {new_instance_id} was already registered: old instance {old_instance}, new_instance {new_instance}"
                    );
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum InstanceControllerCommand {
    Drop {
        offer_id: u64,
        resp_sender: oneshot::Sender<Result<String, StatusCode>>,
    },
    GetAll {
        resp_sender: oneshot::Sender<HashMap<u64, VastInstance>>,
    },
    HandleUnfinishedBusiness,
    VerifyInstance {
        offer_id: u64,
    },
}
