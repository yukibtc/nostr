// Copyright (c) 2022-2023 Yuki Kishimoto
// Copyright (c) 2023-2025 Rust Nostr Developers
// Distributed under the MIT software license

use nostr_lmdb::NostrLmdb;
use nostr_sdk::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let database = NostrLmdb::open("./db/nostr-lmdb").await?;

    // Query events from database
    let filter = Filter::new().kind(Kind::TextNote).limit(10000);
    let now = Instant::now();
    // let txn = database.read_txn()?;
    // let events = database.find_events(&txn, filter)?;
    let events = database.query(filter).await?;
    let elapsed = now.elapsed();
    println!("Got {} events in {} ms", events.len(), elapsed.as_millis());

    Ok(())
}
