use crate::{config::Config, types::VastInstance, vast::VastClient};
use anyhow::{Context, Result};
use log::{error, info, warn};
use std::collections::HashMap;
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

    pub async fn drop(&self, instance_id: u64) -> Result<()> {
        let command = InstanceControllerCommand::Drop { instance_id };
        self.sender.send(command).await?;

        Ok(())
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
}

pub struct InstanceController {
    // mapping instance_id -> instance
    instances: HashMap<u64, VastInstance>,
    instances_to_drop: Vec<VastInstance>,
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
            .create_instances(desired_instances)
            .await
            .context("Initial instance creation")?;
        let instances = instances.into_iter().collect();
        let instances_to_drop = Vec::new();

        let elapsed = start.elapsed().as_secs_f32();
        info!(
            "Created initial {desired_instances} instances in {:.2} seconds",
            elapsed
        );

        Ok(Self {
            instances,
            instances_to_drop,
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
            let start = Instant::now();
            let command_string = format!("{:?}", command);
            match command {
                InstanceControllerCommand::HandleUnfinishedBusiness => {
                    let mut instances_still_not_dropped = Vec::new();

                    for instance in &self.instances_to_drop {
                        let instance_id = instance.instance_id;

                        // TODO: can remove this when we're sure the logic is fine
                        if let None = self.instances.get(&instance_id) {
                            info!("Instances: {:?}", self.instances);
                            info!("Instances to drop: {:?}", self.instances_to_drop);
                            panic!(
                                "id {instance_id} exists in instances_to_drop but not master instance list. Check logic"
                            );
                        }

                        match self.vast_client.drop_instance(instance_id.clone()).await {
                            Ok(_) => {
                                // if it was dropped successfully, remove it from the list of instances
                                match self.instances.remove(&instance_id) {
                                    None => {
                                        warn!(
                                            "Dropped instance_id {instance_id} but it wasn't known to this magister"
                                        );
                                    }
                                    Some(instance) => {
                                        info!("Dropped {instance}");
                                    }
                                }
                            }
                            Err(e) => {
                                warn!(
                                    "Error on attempt to drop {instance}.  Will try again later. {e}"
                                );
                                instances_still_not_dropped.push(instance.clone());
                            }
                        }
                    }

                    self.instances_to_drop = instances_still_not_dropped;

                    self.ensure_sufficient_instances().await;
                }
                InstanceControllerCommand::Drop { instance_id } => {
                    match self.instances.get(&instance_id) {
                        Some(i) => {
                            self.instances_to_drop.push(i.clone());
                        }
                        None => {
                            warn!(
                                "Attempted to drop instance_id {instance_id} but it isn't known to this magister.  Skipping request."
                            );
                            continue;
                        }
                    };
                }
                InstanceControllerCommand::GetAll { resp_sender } => {
                    if let Err(_) = resp_sender.send(self.instances.clone()) {
                        error!("Get all instances response receiver dropped.  Exiting");
                        break;
                    }
                }
            }
            let secs = start.elapsed().as_secs_f64();
            info!(
                "Took {} seconds to process instance_controller command {:?}",
                secs, command_string
            );
        }

        Ok(())
    }

    // requests new instances if we're below config.number_instances
    async fn ensure_sufficient_instances(&mut self) {
        info!(
            "Currently at {} / {} instances",
            self.instances.len(),
            self.config.number_instances
        );

        if self.instances.len() < self.config.number_instances {
            let required_instances = self.config.number_instances - self.instances.len();

            let new_instances = match self
                .vast_client
                .create_instances(required_instances)
                .await
                .context("Request missing instances")
            {
                Ok(x) => x,
                Err(e) => {
                    warn!("Error creating {required_instances} new instances: {e}");
                    return;
                }
            };

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
        instance_id: u64,
    },
    GetAll {
        resp_sender: oneshot::Sender<HashMap<u64, VastInstance>>,
    },
    HandleUnfinishedBusiness,
}
