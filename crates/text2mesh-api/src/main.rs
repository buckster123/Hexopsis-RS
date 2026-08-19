//! text2mesh-api — REST + optional HTMX later.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    text2mesh_api::run().await
}
