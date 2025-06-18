use crate::{
    config::{Config, VastConfig},
    types::VastInstance,
    vast::VastClient,
};
use anyhow::Result;
use log::{error, info, warn};
use std::collections::HashMap;
use tokio::{
    sync::{mpsc, oneshot},
    time::Instant,
};

#[derive(Clone)]
pub struct InstanceControllerClient {
    sender: mpsc::Sender<InstanceControllerCommand>,
}

impl InstanceControllerClient {
    pub async fn new(config: Config) -> Self {
        let vast_client = VastClient::new(config.clone());
        let desired_instances = config.number_instances;
        // TODO: request <desired_instances> of <template>

        let (sender, receiver) = mpsc::channel(100);
        let controller = InstanceController::new(vast_client, config.clone(), receiver);

        tokio::task::spawn(async move { controller.background_event_loop().await });

        Self { sender }
    }
}

pub struct InstanceController {
    // mapping ip:port -> instance
    instances: HashMap<String, VastInstance>,
    vast_client: VastClient,
    receiver: mpsc::Receiver<InstanceControllerCommand>,
    config: Config,
}

impl InstanceController {
    fn new(
        vast_client: VastClient,
        config: Config,
        receiver: mpsc::Receiver<InstanceControllerCommand>,
    ) -> Self {
        let instances = HashMap::new();
        Self {
            instances,
            vast_client,
            receiver,
            config,
        }
    }

    async fn background_event_loop(mut self) -> Result<()> {
        while let Some(command) = self.receiver.recv().await {
            let start = Instant::now();
            let command_string = format!("{:?}", command);
            match command {
                InstanceControllerCommand::Drop { instance_id } => {
                    match self.instances.remove_entry(&instance_id) {
                        None => warn!(
                            "Attempted to drop instance {instance_id} but it isn't known to this magister"
                        ),
                        Some(_) => info!("Dropped instance {instance_id}"),
                    }

                    if let Err(e) = self.vast_client.drop_instance(&instance_id).await {
                        error!("Error dropping instance {instance_id}: {e}");
                    }

                    if self.instances.len() < self.config.number_instances {
                        //let (instance_id, instance) = self.vast_client.new_instance().await;
                    }
                }
                InstanceControllerCommand::GetAll { resp_sender } => {
                    if let Err(_) = resp_sender.send(self.instances.clone()) {
                        error!("Receiver for GetAll command dropped");
                    }
                }
            }

            let secs = start.elapsed().as_secs_f64();
            if secs > 0.5 {
                info!(
                    "Slow execution detected: took {} seconds to process instance_controller command {:?}",
                    secs, command_string
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum InstanceControllerCommand {
    Drop {
        instance_id: String,
    },
    GetAll {
        resp_sender: oneshot::Sender<HashMap<String, VastInstance>>,
    },
}
