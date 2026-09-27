# Testing

```sh
scripts/fetch-pdfium.sh   # once; the engine and app tests need PDFium
cargo test
```

The suite has three layers, one per crate. None of them opens windows on your desktop,
touches your files or needs a network connection.

## refr-core: the model

Plain unit tests with no PDFium, in `crates/refr-core/src/*.rs`:

| Test | Checks |
|---|---|
| `reads_pdfspace_json_written_by_system_text_json` | A workspace written by PdfSpace, computed properties included, loads and round-trips |
| `transforms_round_trip_for_every_rotation_and_crop` | Page ↔ display transforms are exact inverses |
| `two_page_layout_pairs_pages` | Two-page layout placement |
| `undo_restores_clean_state_by_identity` | Undoing to the saved snapshot clears the dirty flag |
| `source_bytes_are_shared_between_snapshots` | History doesn't copy PDF bytes |
| `page_operations` | Insert, duplicate, move and delete |
| `history_is_bounded` | Undo history stops at 100 |
| `invalid_changes_are_rejected_without_history` | A change that fails validation leaves no history entry |
| `wraps_on_width_and_newlines`, `parses_ranges`, `union_and_intersection` | Text wrapping, page ranges and geometry |

## refr-pdf: the engine against real PDFium

`crates/refr-pdf/src/tests.rs` binds PDFium once per test process and uses the generated
sample report, so the tests don't depend on fixture files:

| Test | Checks |
|---|---|
| `sample_opens_with_six_bookmarked_pages` | Import and the sample document |
| `words_are_in_top_left_logical_space` | Word boxes use logical coordinates |
| `search_finds_page_text_and_annotation_text` | Find covers pages and annotations |
| `render_applies_rotation_and_crop` | Bitmap sizes and orientation for rotated and cropped pages |
| `blank_pages_have_no_bitmap` | Blank pages render as nothing |
| `export_keeps_text_searchable_and_applies_edits` | Exported pages keep their text, rotation, crop and comments |
| `png_and_split_exports` | PNG output and ZIP split |
| `logical_and_user_space_round_trip` | The PDF user space ↔ logical space mapping |

## refr: headless UI tests

`crates/refr/src/tests.rs` runs a real `Workbench` in GPUI's test window
(`#[gpui::test]`, `VisualTestContext`). Events are simulated in-process, so nothing reaches
the desktop, and the tests run in well under a second.

| Test | Checks |
|---|---|
| `opens_the_sample_fitted_to_the_window` | Startup, layout and zoom |
| `highlight_tool_marks_the_dragged_words` | Dragging across text makes highlights on the right words |
| `draw_select_move_resize_and_undo` | Drawing, selecting, moving, resizing and undo |
| `ink_on_a_rotated_page_lands_where_it_was_drawn` | Pointer-to-page mapping on rotated pages |
| `text_tool_types_an_annotation` | The inline text editor receives typing and applies it |
| `note_tool_asks_for_the_comment_text` | The comment dialog, and that the Comments panel opens |
| `crop_tool_crops_the_page` | Cropping by dragging |
| `select_tool_copies_dragged_text` | Text selection and ⌘C |
| `command_scroll_zooms_around_the_pointer` | The page point under the pointer stays fixed while zooming |
| `keyboard_shortcuts_reach_the_viewport` | Delete, tool keys and Page Down |
| `workspace_round_trips_through_a_file` | Saving and reopening a workspace |
| `coordinates_round_trip_exactly` (refr-core) | Coordinates survive a save and load bit for bit |
| `about_window_opens_from_the_menu_action_and_closes_with_escape` | The About window and focus afterwards |
| `recovery_copy_is_removed_when_changes_are_undone` | Undoing back to the saved state deletes the recovery copy |
| `saving_a_workspace_removes_the_recovery_copy` | Saving deletes it |
| `recovery_follows_another_document_with_unsaved_changes` | The copy switches to the next document with unsaved changes |
| `restored_recovery_opens_as_unsaved` | **Restore** opens the copy as an unsaved document and keeps the file |
| `discarding_the_recovery_copy_deletes_it` | **Discard** deletes it |
| `quitting_writes_a_pending_recovery_copy_at_once` and `quitting_with_everything_saved_removes_the_recovery_copy` | What happens to the recovery copy on quit |
| `helvetica_outlines_follow_the_pen` | Glyph outlines for rotated text |

Each test gets its own temporary data folder (`data_dir()` in `tests.rs`), passed to
`Workbench::new`. Tests never read or overwrite your recovery copy or recent files, and
tests running in parallel don't see each other's files.

### Writing a UI test

The helpers at the top of `tests.rs` do most of the work:

```rust
#[gpui::test]
fn rectangle_tool_draws_a_rectangle(cx: &mut TestAppContext) {
    let (workbench, cx) = setup(cx);                 // workbench with the sample open
    let doc = doc(&workbench, cx);                   // the active DocumentView
    use_tool(&workbench, cx, PdfTool::Rectangle);
    let from = at(&doc, cx, 0, PointD::new(100.0, 450.0)); // page point → window point
    let to = at(&doc, cx, 0, PointD::new(200.0, 500.0));
    drag(cx, from, to);                              // down, ten moves, up
    let a = annotations(&doc, cx, 0).pop().expect("rectangle added");
    assert_eq!(a.kind, AnnotationKind::Rectangle);
}
```

- Express positions in **page coordinates** and convert them with `at`, so tests don't
  depend on window size or zoom.
- Call `cx.run_until_parked()` after events that start background work, such as renders,
  searches and dialogs.
- Timers, such as the recovery copy's one-second delay, use GPUI's test clock. Move it
  forward with `cx.executor().advance_clock(duration)`.
- Answer macOS alerts with `cx.simulate_prompt_answer("Restore")`.
- Use `cx.simulate_keystrokes("cmd-z")` for shortcuts and `cx.simulate_input("text")` for
  typing. Both go through the real key bindings.
- Use `cx.dispatch_action(SomeAction)` for menu commands.

## Checking rendering by eye

Some things are easier to judge by looking at them:

```sh
cargo run -p refr-pdf --example dump -- /tmp/refr-dump
open /tmp/refr-dump
```

This writes PNGs of sample pages, an annotated export and a rotated page, so you can compare
the viewer with the exported PDF.

## Before you commit

```sh
cargo test
cargo build -p refr 2>&1 | grep -E "^warning" || echo "no warnings"
```

A change should build without warnings and keep every test passing. Add a test when you fix
a bug or add behavior.
