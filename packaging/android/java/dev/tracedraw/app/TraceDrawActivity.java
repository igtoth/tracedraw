// The Android app's activity (docs/behavior/android.md): the system's
// NativeActivity, which loads libtracedraw.so and hands it the window and
// the input, plus the few things the editor can only do from Java: the
// system document picker (Storage Access Framework), reading and writing
// the documents it returns, the system clipboard, the print service, the
// share sheet and "open with" (files served by SharedFiles), and an
// invisible view the on-screen keyboard types into (TextInputView). The Rust side calls
// the public methods through JNI from its own thread; a picked document
// comes back through the native method nativeDocumentResult. Built by
// scripts/android/build.sh with javac and d8 into classes.dex.
package dev.tracedraw.app;

import android.app.NativeActivity;
import android.content.ActivityNotFoundException;
import android.content.ClipData;
import android.content.ClipboardManager;
import android.content.ContentResolver;
import android.content.Context;
import android.content.Intent;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.print.PageRange;
import android.print.PrintAttributes;
import android.print.PrintDocumentAdapter;
import android.print.PrintDocumentInfo;
import android.os.Process;
import android.print.PrintManager;
import android.provider.OpenableColumns;
import android.util.Log;
import android.view.ViewGroup;
import android.view.inputmethod.InputMethodManager;

import java.io.ByteArrayOutputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.InputStream;
import java.io.OutputStream;
import java.util.ArrayList;
import java.util.concurrent.Callable;
import java.util.concurrent.FutureTask;
import java.util.concurrent.TimeUnit;

public class TraceDrawActivity extends NativeActivity {
    private static final String TAG = "tracedraw";

    static {
        // The same library NativeActivity loads (android.app.lib_name), so
        // that the native methods below resolve to its symbols.
        System.loadLibrary("tracedraw");
    }

    /**
     * The documents picked for the request `request` (a code the Rust side
     * chose), as newline-separated lists of the same length: the content
     * URIs, their display names and their MIME types (an empty entry when
     * unknown). All three are null when the picker was cancelled or failed.
     * Implemented in Rust (android.rs).
     */
    private native void nativeDocumentResult(int request, String uris, String names, String mimes);

    /**
     * What the on-screen keyboard's input method did in TextInputView: the
     * text it composes (empty: the composition ends), the text it commits,
     * and the characters it deletes before and after the caret.
     * Implemented in Rust (android.rs).
     */
    private native void nativeImeCompose(String text);
    private native void nativeImeCommit(String text);
    private native void nativeImeDelete(int before, int after);

    /** The view the on-screen keyboard types into. */
    private TextInputView textInput;

    /**
     * Wait, at most `timeoutMs`, until the editor's thread has let go of
     * the activity's window, which it does after backing the drawings up
     * (eframe's save on suspend). Implemented in Rust (android.rs).
     */
    private native void nativeAwaitWindowGone(long timeoutMs);

    /** How long the end of the activity waits for the backup. */
    private static final long BACKUP_WAIT_MS = 10000;

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        textInput = new TextInputView(this);
        addContentView(textInput, new ViewGroup.LayoutParams(1, 1));
    }

    /**
     * The activity ends: relaunched by the system (a change of its
     * resource overlays, which no configChanges entry can take over, or of
     * the screen density) or finished. NativeActivity.onDestroy then waits
     * for the editor's thread to return from android_main, which it never
     * does (winit runs one event loop per process and cannot start
     * another), so the app froze for good with the system's starting
     * window on the screen. Instead, once the editor has given up the
     * window (and backed the drawings up while doing so, which happens
     * inside super.onDestroy), the process ends; the manifest's
     * stateNotNeeded makes the system start the activity again in a new
     * process, which offers the backed-up drawings.
     */
    @Override
    protected void onDestroy() {
        final boolean relaunch = isChangingConfigurations();
        Thread end = new Thread(new Runnable() {
            @Override
            public void run() {
                try {
                    nativeAwaitWindowGone(BACKUP_WAIT_MS);
                } catch (UnsatisfiedLinkError e) {
                    // The editor's thread has not started: nothing to back up.
                    Log.w(TAG, "ending: " + e);
                }
                Log.i(TAG, relaunch
                        ? "the activity is relaunched: the process ends and the system starts it again"
                        : "the activity ends: the process ends");
                Process.killProcess(Process.myPid());
            }
        }, "tracedraw-end");
        end.setDaemon(true);
        end.start();
        super.onDestroy();
    }

    // ----- the on-screen keyboard ---------------------------------------------

    /**
     * Show the on-screen keyboard, typing into the editor (`on`: a text or
     * a field takes typed text), or hide it.
     */
    public void showTextInput(final boolean on) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                InputMethodManager imm = (InputMethodManager) getSystemService(Context.INPUT_METHOD_SERVICE);
                if (imm == null || textInput == null) {
                    return;
                }
                if (on) {
                    textInput.requestFocus();
                    imm.restartInput(textInput);
                    imm.showSoftInput(textInput, 0);
                } else {
                    // A composition left open ends with the editing.
                    imm.restartInput(textInput);
                    imm.hideSoftInputFromWindow(textInput.getWindowToken(), 0);
                }
            }
        });
    }

    void imeCompose(String text) {
        try {
            nativeImeCompose(text);
        } catch (UnsatisfiedLinkError e) {
            Log.w(TAG, "keyboard: " + e);
        }
    }

    void imeCommit(String text) {
        try {
            nativeImeCommit(text);
        } catch (UnsatisfiedLinkError e) {
            Log.w(TAG, "keyboard: " + e);
        }
    }

    void imeDelete(int before, int after) {
        try {
            nativeImeDelete(before, after);
        } catch (UnsatisfiedLinkError e) {
            Log.w(TAG, "keyboard: " + e);
        }
    }

    // ----- the document picker -----------------------------------------------

    /**
     * Show the system's document picker for one document (or several when
     * `multiple`) of any type; the editor tells the type from the name.
     * The result comes through nativeDocumentResult.
     */
    public void openDocument(final int request, final boolean multiple) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    Intent intent = new Intent(Intent.ACTION_OPEN_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType("*/*");
                    intent.putExtra(Intent.EXTRA_ALLOW_MULTIPLE, multiple);
                    startActivityForResult(intent, request);
                } catch (Exception e) {
                    Log.w(TAG, "the document picker could not open", e);
                    nativeDocumentResult(request, null, null, null);
                }
            }
        });
    }

    /**
     * Show the system's picker for a place to save a new document named
     * `name` of type `mime`. The result comes through nativeDocumentResult.
     */
    public void createDocument(final int request, final String mime, final String name) {
        runOnUiThread(new Runnable() {
            @Override
            public void run() {
                try {
                    Intent intent = new Intent(Intent.ACTION_CREATE_DOCUMENT);
                    intent.addCategory(Intent.CATEGORY_OPENABLE);
                    intent.setType(mime == null || mime.isEmpty() ? "application/octet-stream" : mime);
                    intent.putExtra(Intent.EXTRA_TITLE, name);
                    startActivityForResult(intent, request);
                } catch (Exception e) {
                    Log.w(TAG, "the save picker could not open", e);
                    nativeDocumentResult(request, null, null, null);
                }
            }
        });
    }

    @Override
    protected void onActivityResult(int requestCode, int resultCode, Intent data) {
        super.onActivityResult(requestCode, resultCode, data);
        if (resultCode != RESULT_OK || data == null) {
            nativeDocumentResult(requestCode, null, null, null);
            return;
        }
        ArrayList<Uri> uris = new ArrayList<Uri>();
        if (data.getData() != null) {
            uris.add(data.getData());
        }
        ClipData clip = data.getClipData();
        if (clip != null) {
            for (int i = 0; i < clip.getItemCount(); i++) {
                Uri u = clip.getItemAt(i).getUri();
                if (u != null && !uris.contains(u)) {
                    uris.add(u);
                }
            }
        }
        if (uris.isEmpty()) {
            nativeDocumentResult(requestCode, null, null, null);
            return;
        }
        // Keep the right to read (and write, when granted) the document
        // after the process restarts, so the recent list reopens it.
        int flags = data.getFlags()
            & (Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        StringBuilder u = new StringBuilder();
        StringBuilder n = new StringBuilder();
        StringBuilder m = new StringBuilder();
        for (int i = 0; i < uris.size(); i++) {
            Uri uri = uris.get(i);
            try {
                if (flags != 0) {
                    getContentResolver().takePersistableUriPermission(uri, flags);
                }
            } catch (Exception e) {
                Log.i(TAG, "no persistable permission for " + uri);
            }
            if (i > 0) {
                u.append('\n');
                n.append('\n');
                m.append('\n');
            }
            u.append(uri.toString());
            n.append(displayName(uri).replace('\n', ' '));
            String mime = null;
            try {
                mime = getContentResolver().getType(uri);
            } catch (Exception e) {
                // Unknown type: the editor goes by the name.
            }
            m.append(mime == null ? "" : mime.replace('\n', ' '));
        }
        nativeDocumentResult(requestCode, u.toString(), n.toString(), m.toString());
    }

    /** The name the document's provider shows for it; its last path segment when it has none. */
    private String displayName(Uri uri) {
        Cursor cursor = null;
        try {
            cursor = getContentResolver().query(uri, new String[] {OpenableColumns.DISPLAY_NAME}, null, null, null);
            if (cursor != null && cursor.moveToFirst()) {
                int column = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME);
                if (column >= 0) {
                    String name = cursor.getString(column);
                    if (name != null && !name.isEmpty()) {
                        return name;
                    }
                }
            }
        } catch (Exception e) {
            Log.i(TAG, "no display name for " + uri);
        } finally {
            if (cursor != null) {
                cursor.close();
            }
        }
        String last = uri.getLastPathSegment();
        if (last == null || last.isEmpty()) {
            return "document";
        }
        // Document ids look like "primary:Pictures/a.png": the name is the
        // part after the last separator.
        int cut = Math.max(last.lastIndexOf('/'), last.lastIndexOf(':'));
        return cut >= 0 && cut + 1 < last.length() ? last.substring(cut + 1) : last;
    }

    // ----- reading and writing documents --------------------------------------

    /** The whole content of the document at `uri`; null when it cannot be read. */
    public byte[] readDocument(String uri) {
        InputStream in = null;
        try {
            in = getContentResolver().openInputStream(Uri.parse(uri));
            if (in == null) {
                return null;
            }
            ByteArrayOutputStream out = new ByteArrayOutputStream();
            byte[] buffer = new byte[64 * 1024];
            int got;
            while ((got = in.read(buffer)) != -1) {
                out.write(buffer, 0, got);
            }
            return out.toByteArray();
        } catch (Exception e) {
            Log.w(TAG, "cannot read " + uri + ": " + e);
            return null;
        } finally {
            close(in);
        }
    }

    /**
     * Replace the content of the document at `uri` with `data`; false when
     * it cannot be written. The document is truncated first ("wt"); a
     * provider without that mode gets a plain write.
     */
    public boolean writeDocument(String uri, byte[] data) {
        ContentResolver resolver = getContentResolver();
        OutputStream out = null;
        try {
            try {
                out = resolver.openOutputStream(Uri.parse(uri), "wt");
            } catch (Exception e) {
                out = resolver.openOutputStream(Uri.parse(uri), "w");
            }
            if (out == null) {
                return false;
            }
            out.write(data);
            out.flush();
            return true;
        } catch (Exception e) {
            Log.w(TAG, "cannot write " + uri + ": " + e);
            return false;
        } finally {
            close(out);
        }
    }

    private static void close(java.io.Closeable c) {
        if (c != null) {
            try {
                c.close();
            } catch (Exception e) {
                // Nothing left to do with it.
            }
        }
    }

    // ----- printing, sharing and "open with" (#1398) ----------------------------

    /**
     * Hand the PDF at `path` (one of the app's files) to the system's print
     * service as the job `job`; its dialog picks the printer, the copies
     * and the paper. Whether the dialog opened.
     */
    public boolean printFile(final String path, final String job) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                PrintManager printing = (PrintManager) getSystemService(Context.PRINT_SERVICE);
                final File file = new File(path);
                if (printing == null || !file.isFile()) {
                    return Boolean.FALSE;
                }
                printing.print(job, new PdfAdapter(file, job), null);
                return Boolean.TRUE;
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    /** Hands a ready PDF to the print service as it is. */
    private static final class PdfAdapter extends PrintDocumentAdapter {
        private final File file;
        private final String name;

        PdfAdapter(File file, String name) {
            this.file = file;
            this.name = name;
        }

        @Override
        public void onLayout(PrintAttributes oldAttributes, PrintAttributes newAttributes,
                CancellationSignal cancel, LayoutResultCallback callback, Bundle extras) {
            if (cancel != null && cancel.isCanceled()) {
                callback.onLayoutCancelled();
                return;
            }
            PrintDocumentInfo info = new PrintDocumentInfo.Builder(name.endsWith(".pdf") ? name : name + ".pdf")
                .setContentType(PrintDocumentInfo.CONTENT_TYPE_DOCUMENT)
                .setPageCount(PrintDocumentInfo.PAGE_COUNT_UNKNOWN)
                .build();
            callback.onLayoutFinished(info, oldAttributes == null || !oldAttributes.equals(newAttributes));
        }

        @Override
        public void onWrite(PageRange[] pages, ParcelFileDescriptor destination,
                CancellationSignal cancel, WriteResultCallback callback) {
            InputStream in = null;
            FileOutputStream out = null;
            try {
                in = new FileInputStream(file);
                out = new FileOutputStream(destination.getFileDescriptor());
                byte[] buffer = new byte[64 * 1024];
                int got;
                while ((got = in.read(buffer)) != -1) {
                    if (cancel != null && cancel.isCanceled()) {
                        callback.onWriteCancelled();
                        return;
                    }
                    out.write(buffer, 0, got);
                }
                out.flush();
                callback.onWriteFinished(new PageRange[] {PageRange.ALL_PAGES});
            } catch (Exception e) {
                Log.w(TAG, "print: " + e);
                callback.onWriteFailed(e.toString());
            } finally {
                close(in);
                close(out);
            }
        }
    }

    /**
     * Offer the file at `path` (one of the app's files, of type `mime`) to
     * another app through the share sheet, with `subject` for a mail;
     * `mail` limits the choice to mail apps. Whether the sheet opened.
     */
    public boolean shareFile(final String path, final String mime, final String subject, final boolean mail) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                Uri uri = SharedFiles.uriFor(TraceDrawActivity.this, new File(path));
                if (uri == null) {
                    Log.w(TAG, "share: " + path + " is not one of the app's files");
                    return Boolean.FALSE;
                }
                Intent send = new Intent(Intent.ACTION_SEND);
                send.setType(mime);
                send.putExtra(Intent.EXTRA_STREAM, uri);
                send.putExtra(Intent.EXTRA_SUBJECT, subject);
                send.setClipData(ClipData.newRawUri(subject, uri));
                send.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                if (mail) {
                    // Only the apps that send mail.
                    send.setSelector(new Intent(Intent.ACTION_SENDTO, Uri.parse("mailto:")));
                }
                try {
                    startActivity(Intent.createChooser(send, subject));
                    return Boolean.TRUE;
                } catch (ActivityNotFoundException e) {
                    Log.w(TAG, "share: no app takes " + mime);
                    return Boolean.FALSE;
                }
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    /**
     * Open the file at `path` (one of the app's files, of type `mime`) in
     * an app that edits it, with the right to write it back; the system
     * asks which app when there are several. False when none edits it.
     */
    public boolean editFile(final String path, final String mime) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                Uri uri = SharedFiles.uriFor(TraceDrawActivity.this, new File(path));
                if (uri == null) {
                    Log.w(TAG, "edit: " + path + " is not one of the app's files");
                    return Boolean.FALSE;
                }
                Intent edit = new Intent(Intent.ACTION_EDIT);
                edit.setDataAndType(uri, mime);
                edit.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION | Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
                try {
                    startActivity(edit);
                    return Boolean.TRUE;
                } catch (ActivityNotFoundException e) {
                    Log.i(TAG, "edit: no app edits " + mime);
                    return Boolean.FALSE;
                }
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    /**
     * Show `target` in the app the system has for it: a web address (or
     * any other URI with a scheme, such as a picked document's content
     * URI), or one of the app's files (an absolute path), of type `mime`
     * when it is not empty. False when no app opens it.
     */
    public boolean viewTarget(final String target, final String mime) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                Intent view = new Intent(Intent.ACTION_VIEW);
                if (target.startsWith("/")) {
                    Uri uri = SharedFiles.uriFor(TraceDrawActivity.this, new File(target));
                    if (uri == null) {
                        Log.w(TAG, "view: " + target + " is not one of the app's files");
                        return Boolean.FALSE;
                    }
                    String type = mime == null || mime.isEmpty() ? SharedFiles.mimeOf(target) : mime;
                    view.setDataAndType(uri, type);
                    view.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                } else {
                    // A web address, or a document the picker returned,
                    // whose grant this app holds and passes on to read it.
                    Uri uri = Uri.parse(target);
                    if (mime == null || mime.isEmpty()) {
                        view.setData(uri);
                    } else {
                        view.setDataAndType(uri, mime);
                    }
                    if ("content".equals(uri.getScheme())) {
                        view.addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION);
                    }
                }
                try {
                    startActivity(view);
                    return Boolean.TRUE;
                } catch (ActivityNotFoundException e) {
                    Log.i(TAG, "view: no app opens " + target);
                    return Boolean.FALSE;
                }
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    // ----- the system clipboard -----------------------------------------------

    /** The clipboard's text, or null when it holds none (or the app is not in front). */
    public String getClipboardText() {
        return onUiThread(new Callable<String>() {
            @Override
            public String call() {
                ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null || !clipboard.hasPrimaryClip()) {
                    return null;
                }
                ClipData clip = clipboard.getPrimaryClip();
                if (clip == null || clip.getItemCount() == 0) {
                    return null;
                }
                // A document (a picture copied in a gallery or a browser)
                // is no text: its URI would paste as one (#1403).
                for (int i = 0; i < clip.getItemCount(); i++) {
                    ClipData.Item item = clip.getItemAt(i);
                    if (item.getUri() != null && item.getText() == null) {
                        continue;
                    }
                    CharSequence text = item.coerceToText(TraceDrawActivity.this);
                    if (text != null && text.length() > 0) {
                        return text.toString();
                    }
                }
                return null;
            }
        }, null);
    }

    /**
     * The documents on the clipboard (pictures and files other apps
     * copied), one per line as "uri TAB mime TAB name"; null when there is
     * none. Reading the clip grants this app the right to read them.
     */
    public String getClipboardDocuments() {
        return onUiThread(new Callable<String>() {
            @Override
            public String call() {
                ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null || !clipboard.hasPrimaryClip()) {
                    return null;
                }
                ClipData clip = clipboard.getPrimaryClip();
                if (clip == null) {
                    return null;
                }
                StringBuilder out = new StringBuilder();
                for (int i = 0; i < clip.getItemCount(); i++) {
                    Uri uri = clip.getItemAt(i).getUri();
                    if (uri == null) {
                        continue;
                    }
                    String mime = null;
                    try {
                        mime = getContentResolver().getType(uri);
                    } catch (Exception e) {
                        // Unknown type: the editor goes by the name.
                    }
                    if (mime == null && clip.getDescription() != null
                        && clip.getDescription().getMimeTypeCount() > 0) {
                        mime = clip.getDescription().getMimeType(0);
                    }
                    if (out.length() > 0) {
                        out.append('\n');
                    }
                    out.append(uri.toString().replace('\t', ' ').replace('\n', ' '));
                    out.append('\t');
                    out.append(mime == null ? "" : mime.replace('\t', ' ').replace('\n', ' '));
                    out.append('\t');
                    out.append(displayName(uri).replace('\t', ' ').replace('\n', ' '));
                }
                return out.length() == 0 ? null : out.toString();
            }
        }, null);
    }

    /** Put `text` on the clipboard; whether it went there. */
    public boolean setClipboardText(final String text) {
        Boolean done = onUiThread(new Callable<Boolean>() {
            @Override
            public Boolean call() {
                ClipboardManager clipboard = (ClipboardManager) getSystemService(Context.CLIPBOARD_SERVICE);
                if (clipboard == null) {
                    return Boolean.FALSE;
                }
                clipboard.setPrimaryClip(ClipData.newPlainText("TraceDraw", text));
                return Boolean.TRUE;
            }
        }, Boolean.FALSE);
        return done != null && done.booleanValue();
    }

    /**
     * Run `task` on the UI thread and wait for its result (two seconds at
     * most); `fallback` when it fails or takes longer. The caller is the
     * editor's own thread, so the UI thread is free to run it.
     */
    private <T> T onUiThread(Callable<T> task, T fallback) {
        FutureTask<T> future = new FutureTask<T>(task);
        runOnUiThread(future);
        try {
            return future.get(2, TimeUnit.SECONDS);
        } catch (Exception e) {
            Log.w(TAG, "UI thread call failed: " + e);
            return fallback;
        }
    }
}
