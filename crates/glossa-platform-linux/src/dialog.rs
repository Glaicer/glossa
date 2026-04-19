use gtk::{
    init as gtk_init,
    prelude::*,
    ButtonsType, DialogFlags, MessageDialog, MessageType, Window,
};

#[derive(Debug)]
pub enum DialogError {
    GtkInit(String),
}

impl std::fmt::Display for DialogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GtkInit(error) => write!(f, "failed to initialize GTK dialog runtime: {error}"),
        }
    }
}

impl std::error::Error for DialogError {}

pub fn show_fatal_error_dialog(title: &str, message: &str) -> Result<(), DialogError> {
    show_message_dialog(title, message, MessageType::Error)
}

pub(crate) fn show_message_dialog(
    title: &str,
    message: &str,
    message_type: MessageType,
) -> Result<(), DialogError> {
    gtk_init().map_err(|error| DialogError::GtkInit(error.to_string()))?;

    let dialog = MessageDialog::new(
        None::<&Window>,
        DialogFlags::MODAL,
        message_type,
        ButtonsType::Close,
        message,
    );
    dialog.set_title(title);
    let _ = dialog.run();
    dialog.close();

    Ok(())
}
