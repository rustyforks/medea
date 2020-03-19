//! Medea media server.

// TODO: Remove `clippy::must_use_candidate` once the issue below is resolved:
//       https://github.com/rust-lang/rust-clippy/issues/4779
#![allow(clippy::module_name_repetitions, clippy::must_use_candidate)]

#[macro_use]
pub mod utils;
pub mod api;
pub mod conf;
pub mod log;
pub mod media;
pub mod shutdown;
pub mod signalling;
pub mod turn;

use std::sync::Arc;

use actix::{Actor, Addr};

use crate::{
    api::control::callback::{
        clients::CallbackClientFactoryImpl,
        metrics_callback_service::MetricsCallbacksService,
        service::CallbackService,
    },
    conf::Conf,
    turn::{coturn_metrics::CoturnMetrics, TurnAuthService},
};

/// Global application context.
#[derive(Clone, Debug)]
pub struct AppContext {
    /// [Medea] configuration.
    ///
    /// [Medea]: https://github.com/instrumentisto/medea
    pub config: Arc<Conf>,

    /// Reference to [`TurnAuthService`].
    pub turn_service: Arc<dyn TurnAuthService>,

    /// Service for sending [`CallbackEvent`]s.
    ///
    /// [`CallbackEvent`]: crate::api::control::callbacks::CallbackEvent
    pub callbacks: CallbackService<CallbackClientFactoryImpl>,

    /// Service which responsible for the `Peer` metrics based Control API
    /// callbacks.
    ///
    /// Will be used for the all [`Room`]s.
    pub metrics_service: Addr<MetricsCallbacksService>,

    /// Service which is responsible for processing [`PeerConnection`]'s
    /// metrics received from the Coturn.
    pub coturn_metrics: Addr<CoturnMetrics>,
}

impl AppContext {
    /// Creates new [`AppContext`].
    #[inline]
    pub fn new(config: Conf, turn: Arc<dyn TurnAuthService>) -> Self {
        let metrics_service = MetricsCallbacksService::new().start();
        let coturn_metrics =
            CoturnMetrics::new(&config.turn, metrics_service.clone())
                .unwrap()
                .start();

        Self {
            config: Arc::new(config),
            turn_service: turn,
            callbacks: CallbackService::default(),
            metrics_service,
            coturn_metrics,
        }
    }
}
