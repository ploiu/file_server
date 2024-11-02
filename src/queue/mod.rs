use std::future::Future;
use std::sync::{Arc, Mutex};

use std::time::Instant;

#[cfg(not(test))]
use crate::config::FILE_SERVER_CONFIG;
use lapin::{Channel, Connection};

struct RabbitProvider {
    /// the connection to the rabbit mq
    connection: Connection,
    /// the channel that we will be consuming messages from / publishing messages to
    channel: Channel,
}

/// sets up a long-running consumer job that invokes the passed [function](Fn)
/// whenever there are items in the rabbit queue
/// * `last_request_time` - the last time a request was made. A preview will not be generated as long as this value is less than the configured `FilePreview.sleepTimeMillis` value
/// * `function` - the async function to be called on the value consumed from the queue. It must take the data
///   as a [String] and output `true` if the operation was a success, and `false` if the operation was a failure
///   That boolean status will be used to determine if the rabbit message should be acknowledged or not
#[cfg(any(not(test), rust_analyzer))]
pub fn file_preview_consumer<F, Fut>(last_request_time: &Arc<Mutex<Instant>>, function: F)
where
    F: Fn(String) -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
    use lapin::options::BasicNackOptions;
    use lapin::options::{BasicAckOptions, BasicConsumeOptions};
    use lapin::types::FieldTable;

    use rocket::futures::StreamExt;

    use std::time::Duration;

    let config = FILE_SERVER_CONFIG.clone();
    if config.rabbit_mq.enabled {
        // using as_ref here because I definitely do _not_ want to clone the rabbit connection
        let provider = RABBIT_PROVIDER.as_ref().unwrap();
        let last_request_time = last_request_time.clone();
        async_global_executor::spawn(async move {
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
                let time_since_last_request = match last_request_time.lock() {
                    Ok(lock) => lock.elapsed().as_millis(),
                    Err(mut e) => {
                        // we're poisoned. This is not likely given how we use this mutex, but we need to reset its state
                        **e.get_mut() = Instant::now();
                        last_request_time.clear_poison();
                        0
                    }
                } as u32;
                if time_since_last_request <= config.file_preview.sleep_time_millis {
                    log::info!(
                        "Not generating previews since the time since last request is only {:?}",
                        time_since_last_request
                    );
                    // we haven't waited enough time since the last request, so unack the message, sleep, and then skip this item
                    delivery
                        .nack(BasicNackOptions {
                            multiple: false,
                            requeue: true,
                        })
                        .await
                        .unwrap();
                    std::thread::sleep(Duration::from_millis(
                        config.file_preview.sleep_time_millis as u64,
                    ));
                    continue;
                }
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
    use std::backtrace::Backtrace;

    use lapin::{options::BasicPublishOptions, BasicProperties};

    if !FILE_SERVER_CONFIG.clone().rabbit_mq.enabled {
        return;
    }

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
            "Failed to publish message {message} to queue {queue_name}. Exception is {e:?}\n{}",
            Backtrace::force_capture()
        );
    }
}

/// should only be called if RabbitConfig.enabled = true
#[cfg(any(not(test), rust_analyzer))]
impl RabbitProvider {
    fn init() -> Self {
        use lapin::options::QueueDeclareOptions;
        use lapin::types::FieldTable;
        use lapin::ConnectionProperties;

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
            let queue_options = QueueDeclareOptions {
                passive: false,
                durable: true,
                exclusive: false,
                auto_delete: false,
                nowait: false,
            };
            channel
                .queue_declare("icon_gen", queue_options, FieldTable::default())
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
static RABBIT_PROVIDER: once_cell::sync::Lazy<Option<RabbitProvider>> =
    once_cell::sync::Lazy::new(|| {
        let config = FILE_SERVER_CONFIG.clone();
        if config.rabbit_mq.enabled {
            Some(RabbitProvider::init())
        } else {
            None
        }
    });

// ---------------------------- test implementations that don't start up rabbit

#[cfg(all(test, not(rust_analyzer)))]
pub fn file_preview_consumer<F, Fut>(_: &Arc<Mutex<Instant>>, _: F)
where
    F: Fn(String) -> Fut + Send + 'static,
    Fut: Future<Output = bool> + Send,
{
}

#[cfg(all(test, not(rust_analyzer)))]
pub fn publish_message(_: &str, _: &String) {}
