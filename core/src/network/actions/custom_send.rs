extern crate futures;
use crate::{
    action::{Action, ActionWrapper, DirectMessageData},
    context::Context,
    instance::dispatch_action,
    network::direct_message::{CustomDirectMessage, DirectMessage},
};
use futures::{
    future::Future,
    task::{LocalWaker, Poll},
};
use holochain_core_types::{cas::content::Address, error::HolochainError};
use snowflake::ProcessUniqueId;
use std::{pin::Pin, sync::Arc};

/// SendDirectMessage Action Creator for custom (=app) messages
/// This triggers the network module to open a synchronous node-to-node connection
/// by sending the given CustomDirectMessage and preparing to receive a response.
pub async fn custom_send(
    to_agent: Address,
    custom_direct_message: CustomDirectMessage,
    context: &Arc<Context>,
) -> Result<String, HolochainError> {
    let id = ProcessUniqueId::new().to_string();
    let direct_message = DirectMessage::Custom(custom_direct_message);
    let direct_message_data = DirectMessageData {
        address: to_agent,
        message: direct_message,
        msg_id: id.clone(),
        is_response: false,
    };
    let action_wrapper = ActionWrapper::new(Action::SendDirectMessage(direct_message_data));
    dispatch_action(context.action_channel(), action_wrapper);

    /* async {
        sleep(Duration::from_secs(60));
        let action_wrapper = ActionWrapper::new(Action::SendDirectMessageTimeout(id.clone()));
        dispatch_action(context.action_channel(), action_wrapper.clone());
    };*/

    await!(SendResponseFuture {
        context: context.clone(),
        id,
    })
}

/// SendResponseFuture waits for a result to show up in NetworkState::custom_direct_message_replys
pub struct SendResponseFuture {
    context: Arc<Context>,
    id: String,
}

impl Future for SendResponseFuture {
    type Output = Result<String, HolochainError>;

    fn poll(self: Pin<&mut Self>, lw: &LocalWaker) -> Poll<Self::Output> {
        let state = self.context.state().unwrap();
        let state = state.network();
        if let Err(error) = state.initialized() {
            return Poll::Ready(Err(HolochainError::ErrorGeneric(error.to_string())));
        }
        //
        // TODO: connect the waker to state updates for performance reasons
        // See: https://github.com/holochain/holochain-rust/issues/314
        //
        lw.wake();
        match state.custom_direct_message_replys.get(&self.id) {
            Some(result) => Poll::Ready(result.clone()),
            _ => Poll::Pending,
        }
    }
}
