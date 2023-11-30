use anyhow::{bail, Context, Result};

use super::{dom::Dom, rpc::RpcMessage};

mod dom;
mod instance;

pub async fn handle_rpc_message(msg: RpcMessage, dom: &mut Dom) -> Result<()> {
    let method = msg.get_method().trim().to_ascii_lowercase();
    let ctx = || format!("failed to deserialize {}", method.as_str());

    // Handle any incoming requests that match a handler method
    if matches!(msg, RpcMessage::Request(_)) {
        let response = match method.as_str() {
            "dom/root" => {
                let req = dom::RootRequest {};
                req.respond_to(msg, dom).await?
            }
            "dom/get" => {
                let req = msg.get_data::<dom::GetRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "dom/children" => {
                let req = msg.get_data::<dom::ChildrenRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "dom/ancestors" => {
                let req = msg.get_data::<dom::AncestorsRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "dom/findbypath" => {
                let req = msg.get_data::<dom::FindByPathRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "dom/findbyquery" => {
                let req = msg.get_data::<dom::FindByQueryRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "instance/insert" => {
                let req = msg.get_data::<instance::InsertRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "instance/rename" => {
                let req = msg.get_data::<instance::RenameRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "instance/delete" => {
                let req = msg.get_data::<instance::DeleteRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            "instance/move" => {
                let req = msg.get_data::<instance::MoveRequest>();
                req.with_context(ctx)?.respond_to(msg, dom).await?
            }
            _ => bail!("unknown request method '{method}'"),
        };

        let mut stdout = tokio::io::stdout();
        response.write_to(&mut stdout).await?;
    }

    // FUTURE: Handle responses for server -> client rpcs?

    Ok(())
}
