use super::CmdResult;
use crate::core::{
    handle,
    validate::{CoreConfigValidator, ValidationErrorKind, ValidationOutcome},
};
use clash_verge_logging::{Type, logging};
use smartstring::alias::String;

pub use crate::core::validate::{ValidationNoticeTarget, handle_validation_notice};

#[tauri::command]
pub async fn script_validate_notice(status: String, msg: String) -> CmdResult {
    handle::Handle::notice_message(status.as_str(), msg.as_str());
    Ok(())
}

#[tauri::command]
pub async fn validate_script_file(file_path: String) -> CmdResult<ValidationOutcome> {
    logging!(info, Type::Config, "Validate script file: {}", file_path);

    match CoreConfigValidator::validate_config_file_outcome(&file_path, None).await {
        Ok(outcome) => {
            handle_validation_notice(&outcome, ValidationNoticeTarget::Script, "script file");
            Ok(outcome)
        }
        Err(e) => {
            let error_msg = e.to_string();
            logging!(error, Type::Config, "Failed to validate script file: {}", error_msg);
            handle::Handle::notice_message("config_validate::process_terminated", &error_msg);
            Ok(ValidationOutcome::invalid(
                ValidationErrorKind::ProcessTerminated,
                error_msg,
            ))
        }
    }
}
