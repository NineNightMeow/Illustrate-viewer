mod color;
mod commands;
mod database;
mod fingerprint;
mod library;
mod metadata;
mod metadata_queue;
pub mod reference;
mod scanner;
mod thumbnail;

use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

#[tauri::command]
fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .manage(scanner::ScanRegistry::default())
        .manage(metadata_queue::MetadataTaskQueue::default())
        .manage(thumbnail::ThumbnailTaskQueue::default())
        .manage(reference::ReferenceWindowRegistry::default())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let startup_result = (|| {
                let database =
                    database::MetadataDatabase::initialize(app.handle()).map_err(|error| {
                        std::io::Error::other(format!(
                            "Metadata database startup failed [{}]: {}",
                            error.code, error.message
                        ))
                    })?;
                let libraries = library::list_library_records(app.handle()).map_err(|error| {
                    std::io::Error::other(format!(
                        "Library store startup failed [{}]: {}",
                        error.code, error.message
                    ))
                })?;
                database.sync_libraries(&libraries).map_err(|error| {
                    std::io::Error::other(format!(
                        "Metadata database library sync failed [{}]: {}",
                        error.code, error.message
                    ))
                })?;
                app.manage(database);

                let database = app.state::<database::MetadataDatabase>().inner().clone();
                app.state::<metadata_queue::MetadataTaskQueue>()
                    .start_workers(app.handle().clone(), database)
                    .map_err(|error| {
                        std::io::Error::other(format!(
                            "Metadata worker startup failed [{}]: {}",
                            error.code, error.message
                        ))
                    })?;

                app.state::<thumbnail::ThumbnailTaskQueue>()
                    .start_workers(app.handle().clone())
                    .map_err(|error| {
                        std::io::Error::other(format!(
                            "Thumbnail worker startup failed [{}]: {}",
                            error.code, error.message
                        ))
                    })?;
                let app_handle = app.handle().clone();
                tauri::async_runtime::spawn(async move {
                    let registry = app_handle.state::<reference::ReferenceWindowRegistry>();
                    reference::restore_references(&app_handle, &registry);
                });
                Ok::<(), std::io::Error>(())
            })();

            if let Err(error) = startup_result {
                eprintln!("Illustrate Viewer startup failed: {error}");
                let message = "Illustrate Viewer could not open its local image metadata.\n\nYour existing data has not been deleted. Check file permissions, file locks, or another running instance, then try again.";
                let app_handle = app.handle().clone();
                app.dialog()
                    .message(message)
                    .title("Illustrate Viewer could not start")
                    .kind(MessageDialogKind::Error)
                    .show(move |_| app_handle.exit(1));
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_version,
            commands::background::choose_custom_background,
            commands::background::prepare_custom_background,
            commands::background::clear_custom_background,
            commands::library::select_folder,
            commands::library::create_library,
            commands::library::remove_library,
            commands::library::list_libraries,
            commands::database::get_metadata_database_status,
            commands::metadata::get_image_metadata,
            commands::metadata::enqueue_metadata_tasks,
            commands::metadata::retry_image_metadata,
            commands::metadata::cancel_metadata_tasks,
            commands::metadata::get_metadata_queue_status,
            commands::color::get_image_color_metadata,
            commands::color::analyze_image_colors,
            commands::color::sample_image_color,
            commands::fingerprint::get_image_fingerprint,
            commands::fingerprint::analyze_image_fingerprint,
            commands::fingerprint::list_duplicate_candidates,
            commands::fingerprint::get_duplicate_indicators,
            commands::favorite::add_favorite,
            commands::favorite::remove_favorite,
            commands::favorite::is_favorite,
            commands::favorite::list_favorites,
            commands::collection::create_collection,
            commands::collection::delete_collection,
            commands::collection::rename_collection,
            commands::collection::list_collections,
            commands::collection::add_image_to_collection,
            commands::collection::remove_image_from_collection,
            commands::collection::list_collection_images,
            commands::collection::list_collection_memberships,
            commands::collection::add_images_to_collections,
            commands::collection::create_collection_with_images,
            commands::smart_collection::create_smart_collection,
            commands::smart_collection::delete_smart_collection,
            commands::smart_collection::rename_smart_collection,
            commands::smart_collection::list_smart_collections,
            commands::smart_collection::list_smart_collection_images,
            commands::tag::create_tag,
            commands::tag::delete_tag,
            commands::tag::rename_tag,
            commands::tag::list_tags,
            commands::tag::add_tag_to_image,
            commands::tag::remove_tag_from_image,
            commands::tag::list_image_tags,
            commands::tag::list_image_tags_for_images,
            commands::tag::list_tag_images,
            commands::search::search_images,
            commands::search::filter_images,
            commands::search::query_image_assets,
            commands::scanner::scan_library,
            commands::thumbnail::clear_thumbnail_cache,
            commands::thumbnail::prepare_image_asset,
            commands::thumbnail::enqueue_thumbnail_tasks,
            commands::thumbnail::sync_gallery_thumbnail_window,
            commands::thumbnail::release_gallery_thumbnail_window,
            commands::thumbnail::get_thumbnail_queue_status,
            commands::thumbnail::cancel_thumbnail_tasks,
            reference::open_reference_window,
            reference::get_reference_asset,
            reference::update_reference_state,
        ])
        .on_window_event(|window, event| {
            let registry = window.state::<reference::ReferenceWindowRegistry>();
            match event {
                tauri::WindowEvent::Moved(_)
                | tauri::WindowEvent::Resized(_)
                | tauri::WindowEvent::ScaleFactorChanged { .. } => {
                    reference::update_reference_geometry(window, &registry);
                }
                tauri::WindowEvent::Focused(is_focused) => {
                    reference::update_reference_focus(window, &registry, *is_focused);
                }
                tauri::WindowEvent::Destroyed => {
                    reference::remove_destroyed_reference(window, &registry);
                }
                _ => {}
            }
        })
        .build(tauri::generate_context!());

    let app = match app {
        Ok(app) => app,
        Err(error) => {
            eprintln!("Illustrate Viewer startup failed while building the application: {error}");
            return;
        }
    };

    app.run(|app, event| {
        if matches!(event, tauri::RunEvent::ExitRequested { .. }) {
            let registry = app.state::<reference::ReferenceWindowRegistry>();
            reference::begin_shutdown(&registry);
            let _ = reference::flush_references(app, &registry);
            app.state::<metadata_queue::MetadataTaskQueue>().shutdown();
            app.state::<thumbnail::ThumbnailTaskQueue>().shutdown();
        }
    });
}
