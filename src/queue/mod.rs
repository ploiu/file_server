use std::time::{Duration, SystemTime};

use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties, Consumer};
use rocket::futures::{FutureExt, StreamExt};
use rocket::yansi::Paint;

use crate::config::FILE_SERVER_CONFIG;

#[cfg(not(test))]
pub fn setup_rabbit_connection() -> Result<(), lapin::Error> {
    let addr = FILE_SERVER_CONFIG.clone().rabbit_mq.address;

    async_global_executor::block_on(async {
        let rabbit_connection = Connection::connect(&addr, ConnectionProperties::default())
            .await
            .unwrap();
        let channel = rabbit_connection.create_channel().await.unwrap();
        let queue = channel
            .queue_declare(
                "icon_gen",
                QueueDeclareOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
        let consumer = channel
            .basic_consume(
                "icon_gen",
                "icon_gen_consumer",
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();
        async_global_executor::spawn(consume_queue(consumer)).detach();
    });

    Ok(())
}

async fn consume_queue(mut consumer: Consumer) {
    let config = FILE_SERVER_CONFIG.clone();
    let (sleep_time, items_to_process) = (
        config.file_preview.sleep_time_millis,
        config.file_preview.items_to_process_per_batch,
    );
    let mut num_left_to_process: u32 = items_to_process;
    while let Some(delivery) = consumer.next().await {
        let delivery = delivery.expect("error in consumer");
        let msg = String::from_utf8(delivery.data.clone()).unwrap();
        log::info!("{msg}");
        delivery
            .ack(BasicAckOptions::default())
            .await
            .expect("ack failed");
        num_left_to_process -= 1;
        if num_left_to_process == 0 {
            num_left_to_process = items_to_process;
            std::thread::sleep(Duration::from_millis(sleep_time as u64));
        }
    }
}

#[cfg(test)]
pub fn setup_rabbit_connection() -> Result<(), ()> {
    Ok(())
}
