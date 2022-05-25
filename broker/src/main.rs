use librumqttd::{async_locallink::construct_broker, Config};
use std::thread;
use tokio::time::{sleep, Duration};

fn main() {
    println!("start!");
    pretty_env_logger::init();
    let config: Config = confy::load_path("config/rumqttd.conf").expect("no conf file");

    let (mut router, console, servers, builder) = construct_broker(config);

    println!("start router now");
    thread::spawn(move || {
        router.start().expect("could start broker");
    });

    let mut runtime = tokio::runtime::Builder::new_multi_thread();
    runtime.enable_all();
    runtime
        .build()
        .expect("tokio Builder failed")
        .block_on(async {
            let (mut tx, _) = builder
                .connect("localclient", 200)
                .await
                .expect("couldnt connect");

            let console_task = tokio::spawn(console);

            tokio::spawn(async move {
                for i in 0..10usize {
                    sleep(Duration::from_millis(1000)).await;
                    let topic = "hello/world";
                    tx.publish(topic, false, vec![i as u8; 1]).await.unwrap();
                }
            });

            println!("await now");

            servers.await;
            // pub_task.await.expect("FAIL pub task");
            // sub_task.await.expect("FAIL sub task");
            console_task.await.expect("FAIL console task");

            println!("YOYO");
        });
}
