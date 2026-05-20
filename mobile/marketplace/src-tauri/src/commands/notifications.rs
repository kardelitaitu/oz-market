use tauri::AppHandle;
use tauri_plugin_notification::{NotificationExt, PermissionState};

#[tauri::command]
pub fn request_notification_permission(app: AppHandle) -> Result<bool, String> {
    let permission = app
        .notification()
        .permission_state()
        .map_err(|e| e.to_string())?;
    match permission {
        PermissionState::Granted => Ok(true),
        PermissionState::Denied => Ok(false),
        PermissionState::Prompt | PermissionState::PromptWithRationale => {
            match app
                .notification()
                .request_permission()
                .map_err(|e| e.to_string())?
            {
                PermissionState::Granted => Ok(true),
                _ => Ok(false),
            }
        }
    }
}

#[tauri::command]
pub fn send_local_notification(app: AppHandle, title: String, body: String) -> Result<(), String> {
    app.notification()
        .builder()
        .title(&title)
        .body(&body)
        .show()
        .map_err(|e| e.to_string())
}
