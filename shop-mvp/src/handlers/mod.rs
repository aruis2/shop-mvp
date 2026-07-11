// =============================================================================
// 🧩 Handlers — fiecare modul e un "capability domain" separat
// =============================================================================
// Niciun handler nu primește AppState direct. Fiecare primește doar
// domain state-ul său (AuthState, ProductState, CartState, OrderState, AdminState).

pub mod admin;
pub mod auth;
pub mod cart;
pub mod orders;
pub mod products;
