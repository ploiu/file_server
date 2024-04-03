use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions,
    QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties};
use once_cell::sync::Lazy;
use rocket::futures::StreamExt;
use std::future::Future;
use std::time::Duration;

use crate::config::FILE_SERVER_CONFIG;

struct RabbitProvider {
    /// the connection to the rabbit mq
    connection: Connection,
    /// the channel that we will be consuming messages from / publishing messages to
    channel: Channel,
}

/// sets up a long-running consumer job that invokes the passed [function](Fn)
/// whenever there are items in the rabbit queue
/// * `function` - the async function to be called on the value consumed from the queue. It must take the data
///   as a [String] and output `true` if the operation was a success, and `false` if the operation was a failure
///   That boolean status will be used to determine if the rabbit message should be acknowledged or not
#[cfg(any(not(test), rust_analyzer))]
pub fn file_preview_consumer<F, Fut>(function: F)
where
    F: Fn(String) -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    let config = FILE_SERVER_CONFIG.clone();
    if config.rabbit_mq.enabled {
        // using as_ref here because I definitely do _not_ want to clone the rabbit connection
        let provider = RABBIT_PROVIDER.as_ref().unwrap();

        let sleep_time = config.file_preview.sleep_time_millis;
        let items_to_process = config.file_preview.items_to_process_per_batch;
        async_global_executor::spawn(async move {
            let mut num_left_to_process: u32 = items_to_process;
            let mut consumer = provider
                .channel
                .basic_consume(
                    "icon_gen",
                    "icon_gen_consumer",
                    BasicConsumeOptions::default(),
                    FieldTable::default(),
                )
                .await
                .unwrap();
            /* we need to be careful about how this runs, because this stream is never empty therefore,
            this while loop will never complete probably not a big idea given that this is designed
            to run on a dedicated raspi, but still */
            while let Some(delivery) = consumer.next().await {
                let delivery = delivery.expect("error in consumer");
                let msg = String::from_utf8(delivery.data.clone()).unwrap();
                if function(msg).await {
                    delivery
                        .ack(BasicAckOptions::default())
                        .await
                        .expect("ack failed");
                } else {
                    log::info!("not acking message because preview generator returned false");
                    delivery
                        .nack(BasicNackOptions {
                            multiple: false,
                            requeue: true,
                        })
                        .await
                        .unwrap();
                }

                num_left_to_process -= 1;
                if num_left_to_process == 0 {
                    num_left_to_process = items_to_process;
                    std::thread::sleep(Duration::from_millis(sleep_time as u64));
                }
            }
        })
        .detach();
    }
}

/// publishes a message to the queue with the passed `queue_name`.
/// failing to publish a message will not return an error, but will log the
/// reason for failure. This is because rabbit is used to offload smaller tasks
/// that aren't strictly necessary for the operation of the file server.
#[cfg(any(not(test), rust_analyzer))]
pub fn publish_message(queue_name: &str, message: &String) {
    let provider = RABBIT_PROVIDER.as_ref().unwrap();
    let channel = &provider.channel;
    let payload: &[u8] = message.as_bytes();
    let res = async_global_executor::block_on(channel.basic_publish(
        "",
        queue_name,
        BasicPublishOptions::default(),
        payload,
        BasicProperties::default(),
    ));
    if let Err(e) = res {
        log::error!(
            "Failed to publish message {message} to queue {queue_name}. Exception is {:?}",
            e
        );
    }
}

/// should only be called if RabbitConfig.enabled = true
#[cfg(any(not(test), rust_analyzer))]
impl RabbitProvider {
    fn init() -> Self {
        let config = FILE_SERVER_CONFIG.clone();
        let (connection, channel) = async_global_executor::block_on(async {
            let rabbit_connection = Connection::connect(
                &config.rabbit_mq.address.unwrap(),
                ConnectionProperties::default(),
            )
            .await
            .unwrap();
            let channel = rabbit_connection.create_channel().await.unwrap();
            // even though this isn't used anywhere, we need to declare the queue or else it won't exist when we go to consume it
            channel
                .queue_declare(
                    "icon_gen",
                    QueueDeclareOptions::default(),
                    FieldTable::default(),
                )
                .await
                .unwrap();
            (rabbit_connection, channel)
        });
        RabbitProvider {
            connection,
            channel,
        }
    }
}

#[cfg(any(not(test), rust_analyzer))]
static RABBIT_PROVIDER: Lazy<Option<RabbitProvider>> = Lazy::new(|| {
    let config = FILE_SERVER_CONFIG.clone();
    return if config.rabbit_mq.enabled {
        Some(RabbitProvider::init())
    } else {
        None
    };
});

// ---------------------------- test implementations that don't start up rabbit

#[cfg(all(test, not(rust_analyzer)))]
pub fn file_preview_consumer<F, Fut>(_: F)
where
    F: Fn(String) -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
}

#[cfg(all(test, not(rust_analyzer)))]
pub fn publish_message(_: &str, _: &String) {}
