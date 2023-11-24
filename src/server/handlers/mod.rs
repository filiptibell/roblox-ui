use anyhow::{bail, Context, Result};

use super::{dom::Dom, rpc::RpcMessage};

mod dom;

pub async fn handle_rpc_message(msg: RpcMessage, dom: &mut Dom) -> Result<()> {
    let method = msg.get_method().trim().to_ascii_lowercase();

    // Handle any incoming requests that match a handler method
    if matches!(msg, RpcMessage::Request(_)) {
        let response = match method.as_str() {
            "dom/root" => {
                let req = dom::RootRequest {};
                req.respond_to(msg, dom).await?
            }
            "dom/get" => {
                let req = msg
                    .get_data::<dom::GetRequest>()
                    .context("failed to deserialize dom/get")?;
                req.respond_to(msg, dom).await?
            }
            "dom/children" => {
                let req = msg
                    .get_data::<dom::ChildrenRequest>()
                    .context("failed to deserialize dom/children")?;
                req.respond_to(msg, dom).await?
            }
            "dom/findByPath" => {
                let req = msg
                    .get_data::<dom::FindByPathRequest>()
                    .context("failed to deserialize dom/findByPath")?;
                req.respond_to(msg, dom).await?
            }
            "dom/findByQuery" => {
                let req = msg
                    .get_data::<dom::FindByQueryRequest>()
                    .context("failed to deserialize dom/findByQuery")?;
                req.respond_to(msg, dom).await?
            }
            _ => bail!("unknown request method '{method}'"),
        };

        let mut stdout = tokio::io::stdout();
        response.write_to(&mut stdout).await?;
    }

    // FUTURE: Handle responses for server -> client rpcs?

    Ok(())
}
