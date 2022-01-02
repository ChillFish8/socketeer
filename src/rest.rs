use poem::Result;
use poem::web::Data;
use poem_openapi::{OpenApi, Object};
use poem_openapi::payload::Json;
use serde_json::Value;
use uuid::Uuid;

use crate::utils::{JsonResponse, SuperUserBearer};
use crate::ws::Event;


#[derive(Object, Debug)]
pub struct EventPayload {
    room_id: Uuid,

    #[oai(rename = "type")]
    type_: String,

    data: Value,
}


pub struct RestApi;


#[OpenApi]
impl RestApi {
    /// Emit Event
    ///
    /// Emits an event to targets clients.
    #[instrument(name = "event-emitter", skip(self, _token, emitter))]
    pub async fn emit_event(
        &self,
        event: Json<EventPayload>,
        emitter: Data<&crate::emitter::EmitterManager>,
        _token: SuperUserBearer,
    ) -> Result<JsonResponse> {
        emitter.emit(
            &event.0.room_id,
            Event {
                type_: event.0.type_,
                data: event.0.data,
            },
        )?;

        Ok(JsonResponse::Ok)
    }
}