use anyhow::{bail, Context, Result};

use roblox_ui_project::Project;

use crate::rpc::RpcMessage;

mod dom;
mod instance;
mod project;
mod util;

pub async fn handle_rpc_message(msg: RpcMessage, project: &Project) -> Result<()> {
    let method = msg.get_method().trim().to_ascii_lowercase();
    let ctx = || format!("failed to deserialize {}", method.as_str());

    if matches!(msg, RpcMessage::Request(_)) {
        let response = match method.as_str() {
            "dom/root" => {
                let req = dom::RootRequest {};
                req.respond_to(msg, project).await?
            }
            "dom/get" => {
                let req = msg.get_data::<dom::GetRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "dom/children" => {
                let req = msg.get_data::<dom::ChildrenRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "dom/ancestors" => {
                let req = msg.get_data::<dom::AncestorsRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "dom/findbypath" => {
                let req = msg.get_data::<dom::FindByPathRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "dom/findbyquery" => {
                let req = msg.get_data::<dom::FindByQueryRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "instance/insert" => {
                let req = msg.get_data::<instance::InsertRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "instance/rename" => {
                let req = msg.get_data::<instance::RenameRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "instance/delete" => {
                let req = msg.get_data::<instance::DeleteRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "instance/move" => {
                let req = msg.get_data::<instance::MoveRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            "project/setfile" => {
                let req = msg.get_data::<project::SetFileRequest>();
                req.with_context(ctx)?.respond_to(msg, project).await?
            }
            _ => bail!("unknown request method '{method}'"),
        };

        let mut stdout = blocking::Unblock::new(std::io::stdout());
        response.write_to(&mut stdout).await?;
    }

    Ok(())
}
