# Recording the demo GIF

`recall-demo.gif` is made by `scripts/record-demo.cjs` from mock mode. That
shows the interface accurately, but not real search: mock mode uses made-up
files, only matches words in file names, and doesn't run the AI models. A
recording of the real app is better. Here's how to make one.

## Before you start

- Install Recall from a recent build, or run it with `npm run tauri dev`
  after `npm run download-models`.
- Start from an empty library, so the recording shows a folder being added.
  Quit Recall and delete its data folder
  (`%APPDATA%\io.github.akanjiolayinka.recall\` on Windows).
- Use only the synthetic files in `test-data/`. Nothing personal should be
  visible: not your files, not your desktop, not other apps' notifications.
  Copying `test-data` to a neutral path such as `C:\Recall demo\test-data`
  keeps your user name out of the folder path shown in the app.
- On Windows, [ScreenToGif](https://www.screentogif.com/) is free and does
  everything below. On macOS, Kap works; on Linux, Peek.
- Size the Recall window to about 1100 × 700 and record just the window.

## What to record

Keep it short, about 20 seconds, with a pause wherever there's something to
read.

1. Start on the empty Search page. Pause a second.
2. Click **Library**, then **Add folder**, and choose the `test-data`
   folder.
3. Let the indexing run until the folder shows its file count, including
   the step where passages are run through the AI model.
4. Click **Search** and type a search that shares no words with the file it
   should find. `groceries to buy` (finds the shopping list) and
   `headset purchase` (finds the receipt image) both work well. Press
   Enter.
5. Pause on the results so the "Similar in meaning" reason is readable.
6. Click **Show source** on the top result and hold on the highlighted
   passage for two or three seconds.
7. Close the dialog. Trim the end so the loop back to the start isn't
   jarring.

## Exporting

- Export as GIF at 10–15 frames per second, 960 pixels wide.
- Keep it under about 5 MB; lower the frame rate or trim pauses if it's
  bigger.
- Save it over `docs/assets/recall-demo.gif`, and update the caption under
  the GIF in `README.md`, since it will no longer be mock mode.
