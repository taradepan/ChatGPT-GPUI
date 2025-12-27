// UI Module
//
// This module contains all user interface components and theming for the chat application.
//
// ## Current Structure:
//
// - `theme`: Color palette and visual styling constants for the application
//   Currently implements a minimalist black & white theme with glass morphism effects
//
// - `chat_window`: Main chat window component with message display and input area
// - `input_box`: Text input component with full editing capabilities
//
// ## Theme Support:
//
// The application uses `WindowBackgroundAppearance::Blurred` to create a glass morphism
// effect with translucent UI elements that show the blurred desktop background.

pub mod chat_window;
pub mod input_box;
pub mod theme;
