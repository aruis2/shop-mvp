// =============================================================================
// 🧩 State — Capability-based domain states
// =============================================================================
// În spirit seL4: fiecare sub-router primește DOAR capabilitățile de care are
// nevoie. Un handler de autentificare NU poate accesa baza de produse.
// Un handler de produse NU poate accesa coșul sau plățile.
//
// Separarea e la nivel de compilator — fiecare `State(domain): State<XxxState>`
// e un tip diferit, nu poți greși.

use std::sync::Arc;
use axum::Router;
use axum::extract::FromRef;
use rust_auth::AuthRepo;
use rust_cart::CartRepo;
use rust_marketplace_orders::OrderRepo;
use rust_marketplace_products::ProductRepo;
use rust_payment::PaymentRepo;
use sqlx::PgPool;

// FromRef e folosit de macro-urile axum, importul e necesar

use crate::render::RenderService;

// ─── Front Controller State (inner router) ─────────────────────────

#[derive(Clone)]
pub(crate) struct FcState {
    pub inner_router: Arc<Router>,
}

// ─── Master state (doar pentru bootstrap, NU expus handlerelor) ──────

#[derive(Clone)]
pub(crate) struct AppState {
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub orders: Arc<dyn OrderRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub max_qty: i32,
    pub db: PgPool,
    pub fc: FcState,
}

// ─── FromRef: convertește AppState → FcState ──────────────────────

impl FromRef<AppState> for FcState {
    fn from_ref(state: &AppState) -> Self {
        state.fc.clone()
    }
}

// ─── FromRef: convertește AppState → domain state ──────────────────

impl FromRef<AppState> for AuthState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            auth: state.auth.clone(),
            cart: state.cart.clone(),
            renderer: state.renderer.clone(),
            site_url: state.site_url.clone(),
        }
    }
}

impl FromRef<AppState> for ProductState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            products: state.products.clone(),
            auth: state.auth.clone(),
            renderer: state.renderer.clone(),
            site_url: state.site_url.clone(),
            db: state.db.clone(),
        }
    }
}

impl FromRef<AppState> for CartState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            cart: state.cart.clone(),
            products: state.products.clone(),
            auth: state.auth.clone(),
            renderer: state.renderer.clone(),
            site_url: state.site_url.clone(),
            max_qty: state.max_qty,
        }
    }
}

impl FromRef<AppState> for OrderState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            orders: state.orders.clone(),
            cart: state.cart.clone(),
            payment: state.payment.clone(),
            auth: state.auth.clone(),
            renderer: state.renderer.clone(),
            site_url: state.site_url.clone(),
        }
    }
}

impl FromRef<AppState> for AdminState {
    fn from_ref(state: &AppState) -> Self {
        Self {
            products: state.products.clone(),
            orders: state.orders.clone(),
            payment: state.payment.clone(),
            auth: state.auth.clone(),
            renderer: state.renderer.clone(),
            site_url: state.site_url.clone(),
            max_qty: state.max_qty,
            db: state.db.clone(),
        }
    }
}

// ─── Domain states — fiecare cu minimul necesar ─────────────────────

/// 🟢 Auth — poate accesa doar autentificare și render
#[derive(Clone)]
pub struct AuthState {
    pub auth: Arc<dyn AuthRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub renderer: RenderService,
    pub site_url: String,
}

/// 🟢 Produse — poate accesa doar produse, auth (read-only) și render
#[derive(Clone)]
pub struct ProductState {
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub db: sqlx::PgPool,
}

/// 🟡 Coș — are nevoie de cart + products + auth (read-only)
#[derive(Clone)]
pub struct CartState {
    pub cart: Arc<dyn CartRepo>,
    pub products: Arc<dyn ProductRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub max_qty: i32,
}

/// 🟠 Comenzi — are nevoie de orders + cart + payment + auth
#[derive(Clone)]
pub struct OrderState {
    pub orders: Arc<dyn OrderRepo>,
    pub cart: Arc<dyn CartRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub site_url: String,
}

/// 🔐 Admin — are nevoie de toate (dar totul prin trait-uri, nu PgPool direct)
/// TODO: `db` (PgPool) e o gaură de securitate — mută migrate_orders în OrderRepo
#[derive(Clone)]
pub struct AdminState {
    pub products: Arc<dyn ProductRepo>,
    pub orders: Arc<dyn OrderRepo>,
    pub payment: Arc<dyn PaymentRepo>,
    pub auth: Arc<dyn AuthRepo>,
    pub renderer: RenderService,
    pub site_url: String,
    pub max_qty: i32,
    pub db: PgPool,
}
