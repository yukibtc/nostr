use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use nostr_sdk::prelude::*;
use smol::net::TcpStream;
use smol::Timer;

#[derive(Debug)]
struct SmolRuntime {}

impl SmolRuntime {
    fn new() -> Self {
        Self {}
    }
}

impl NostrRuntimeSpawn for SmolRuntime {
    fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        smol::spawn(future).detach();
    }
}

impl NostrRuntimeSpawnBlockingTask for SmolRuntime {
    fn spawn_blocking_task_boxed(
        &self,
        task: BoxedBlockingTask,
    ) -> BoxedFuture<Result<BoxedBlockingOutput, SpawnBlockingTaskError>> {
        Box::pin(async move { Ok(smol::unblock(move || task()).await) })
    }
}

impl NostrRuntimeTimer for SmolRuntime {
    fn sleep(&self, duration: Duration) -> Pin<Box<dyn Future<Output = ()> + Send>> {
        Box::pin(async move {
            Timer::after(duration).await;
        })
    }
}

impl NostrRuntimeTcpStream for SmolRuntime {
    fn tcp_connect<'a>(
        &self,
        addr: TcpStreamAddr<'a>,
    ) -> BoxedFuture<'a, Result<BoxedIoStream, std::io::Error>> {
        Box::pin(async move {
            let stream = match addr {
                TcpStreamAddr::SocketAddr(addr) => TcpStream::connect(addr).await?,
                TcpStreamAddr::HostAndPort { host, port } => {
                    TcpStream::connect((host, port)).await?
                }
            };
            Ok(Box::pin(stream) as BoxedIoStream)
        })
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let runtime = Arc::new(SmolRuntime::new());
    global::install_runtime(runtime);

    smol::block_on(async {
        let client = Client::default();
        client.add_relay("wss://relay.damus.io").await?;
        client.add_relay("wss://nos.lol").await?;

        client.connect().await;

        // Stream events from all connected relays
        let filter = Filter::new().kind(Kind::TextNote).limit(100);
        let mut stream = client
            .stream_events(filter)
            .timeout(Duration::from_secs(15))
            .policy(ReqExitPolicy::ExitOnEOSE)
            .await?;

        while let Some((url, res)) = stream.next().await {
            let event = res?;
            println!("Received event from '{url}': {}", event.as_json());
        }

        Ok(())
    })
}
