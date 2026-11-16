"use strict";
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const $ = (id) => document.getElementById(id);
let queue = {
  jobs: [],
  paused: false,
  folder: "",
  setupComplete: false,
  videoContainer: "mp4",
  videoCodec: "auto",
};
let video = null;
let request = 0;
let adding = false;
let savingPreferences = false;
let platform = "macos";
let toastTimer;
let pasteTimer;
let keyboardInput = false;
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
function animateEntry(node, origin = "center top") {
  if (reducedMotion.matches || keyboardInput) return;
  node.getAnimations().forEach((animation) => animation.cancel());
  node.style.transformOrigin = origin;
  node.animate(
    [
      { opacity: 0, transform: "translateY(-5px) scale(.98)" },
      { opacity: 1, transform: "translateY(0) scale(1)" },
    ],
    { duration: 220, easing: "cubic-bezier(.23,1,.32,1)" },
  );
}
document.addEventListener("keydown", () => {
  keyboardInput = true;
  document.body.classList.add("keyboard");
});
document.addEventListener("pointerdown", () => {
  keyboardInput = false;
  document.body.classList.remove("keyboard");
});
let deps = [];
let hasOpened = false;
let startupReady = false;
const jobNodes = new Map();

function toast(message, error = false) {
  clearTimeout(toastTimer);
  $("toast").textContent = String(message);
  $("toast").classList.toggle("error", error);
  $("toast").hidden = false;
  requestAnimationFrame(() => $("toast").classList.add("visible"));
  $("toast").onmouseenter = () => clearTimeout(toastTimer);
  $("toast").onmouseleave = () => {
    toastTimer = setTimeout(hideToast, 2500);
  };
  toastTimer = setTimeout(hideToast, error ? 8000 : 3500);
}
function hideToast() {
  $("toast").classList.remove("visible");
}
function report(error) {
  toast(error, true);
}
function element(tag, className, text) {
  const node = document.createElement(tag);
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
}
function duration(seconds) {
  const minutes = Math.floor(seconds / 60);
  return minutes >= 60
    ? `${Math.floor(minutes / 60)}h ${minutes % 60}m`
    : `${minutes} min`;
}
const sizeNumber = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 1,
});
function formatBytes(bytes) {
  if (!Number.isFinite(bytes) || bytes < 0) return "";
  if (bytes < 1000) return `${Math.round(bytes)} B`;
  if (bytes < 1000 ** 2) return `${sizeNumber.format(bytes / 1000)} KB`;
  if (bytes < 1000 ** 3) return `${sizeNumber.format(bytes / 1000 ** 2)} MB`;
  return `${sizeNumber.format(bytes / 1000 ** 3)} GB`;
}
function size(bytes) {
  return bytes > 0 ? ` · ≈ ${formatBytes(bytes)}` : "";
}
function resetVideo() {
  video = null;
  $("video-card").hidden = true;
  $("thumbnail").removeAttribute("src");
}
function updateDownloadButton() {
  const format = $("quality").value;
  const pending = queue.jobs.some((j) =>
    ["queued", "downloading", "processing"].includes(j.status),
  );
  $("download-label").textContent =
    pending || queue.paused
      ? "Add to Queue"
      : format === "subtitles"
        ? "Download subtitles"
        : format === "mp3"
          ? "Download MP3"
          : "Download";
  $("download").disabled =
    adding ||
    !video ||
    !format ||
    savingPreferences ||
    (format === "subtitles" && !$("subtitle").value);
  $("output-note").textContent =
    format === "mp3"
      ? "MP3 audio"
      : format === "subtitles"
        ? "SRT subtitles"
        : `${(queue.videoContainer || "mp4").toUpperCase()} video${video?.videoCodec === "av1" ? " · AV1" : ""}`;
  const sub = video?.subtitles.find((s) => s.id === $("subtitle").value);
  $("subtitle-note").textContent = sub
    ? sub.automatic
      ? "Auto-generated · SRT"
      : "Creator-provided · SRT"
    : video && !video.subtitles.length
      ? "No subtitles are available for this video."
      : "";
}
async function inspect() {
  const url = $("url").value.trim();
  if (!url) return;
  clearTimeout(pasteTimer);
  const current = ++request;
  resetVideo();
  $("metadata-loading").hidden = false;
  $("inspect").disabled = true;
  $("link-hint").textContent = "Loading available formats and languages…";
  try {
    const result = await invoke("inspect_video", { url });
    if (current !== request) return;
    video = result;
    $("video-title").textContent = video.title;
    $("video-title").title = video.title;
    $("video-byline").textContent =
      `${video.channel} · ${duration(video.duration)}`;
    if (video.thumbnail) {
      $("thumbnail").src = video.thumbnail;
      $("thumbnail").hidden = false;
    } else {
      $("thumbnail").hidden = true;
    }
    $("quality").replaceChildren();
    const videoGroup = element("optgroup");
    videoGroup.label = "Video with audio";
    video.qualities.forEach((quality, index) => {
      const option = element(
        "option",
        "",
        `${quality.label}${index === 0 ? " · Best" : ""}${size(quality.size)}`,
      );
      option.value = quality.id;
      videoGroup.append(option);
    });
    if (video.qualities.length) $("quality").append(videoGroup);
    else {
      const unavailable = new Option(
        "AV1 unavailable · change codec in Settings",
        "",
      );
      unavailable.disabled = true;
      unavailable.selected = true;
      $("quality").append(unavailable);
    }
    const extras = element("optgroup");
    extras.label = "Audio & subtitles";
    for (const [value, label] of [
      ["mp3", "MP3 · Audio only"],
      ["subtitles", "Subtitles only · SRT"],
    ]) {
      const option = element("option", "", label);
      option.value = value;
      option.disabled = value === "subtitles" && video.subtitles.length === 0;
      extras.append(option);
    }
    $("quality").append(extras);
    $("subtitle").replaceChildren(new Option("None", ""));
    for (const automatic of [false, true]) {
      const tracks = video.subtitles.filter((s) => s.automatic === automatic);
      if (!tracks.length) continue;
      const group = element("optgroup");
      group.label = automatic
        ? "Automatic captions & translations"
        : "Creator-provided subtitles";
      tracks.forEach((sub) =>
        group.append(new Option(`${sub.name} (${sub.language})`, sub.id)),
      );
      $("subtitle").append(group);
    }
    $("video-card").hidden = false;
    animateEntry($("video-card"));
    $("link-hint").textContent = "Select a format and optional subtitles.";
    updateDownloadButton();
  } catch (error) {
    if (current === request) {
      $("link-hint").textContent =
        "Could not load this video. Check the link and try again.";
      report(error);
    }
  } finally {
    if (current === request) {
      $("metadata-loading").hidden = true;
      $("inspect").disabled = false;
    }
  }
}

async function addDownload() {
  if (!video || adding) return;
  adding = true;
  updateDownloadButton();
  try {
    queue = await invoke("enqueue", {
      videoId: video.id,
      format: $("quality").value,
      subtitleId: $("subtitle").value || null,
    });
    ++request;
    resetVideo();
    $("url").value = "";
    $("link-hint").textContent = "Added to queue. Paste another link.";
    renderQueue();
    $("url").focus();
    toast(
      queue.paused ? "Added to your paused queue." : "Added to your queue.",
    );
  } catch (error) {
    report(error);
  } finally {
    adding = false;
    updateDownloadButton();
  }
}
async function queueAction(action, id = null) {
  try {
    queue = await invoke("queue_action", { action, id });
    renderQueue();
  } catch (error) {
    report(error);
  }
}
function actionButton(label, symbol, action, id) {
  const button = element("button", "", symbol);
  button.title = label;
  button.setAttribute("aria-label", label);
  button.addEventListener("click", () => queueAction(action, id));
  return button;
}
function createJobNode(job) {
  const node = element("article", "job");
  const top = element("div", "job-top");
  const number = element("span", "job-number");
  const info = element("div", "job-info");
  const title = element("div", "job-title", job.video.title);
  title.title = job.video.title;
  const detail = element("div", "job-detail");
  info.append(title, detail);
  const actions = element("div", "job-actions");
  top.append(number, info, actions);
  const progress = element("div", "job-progress");
  const track = element("div", "progress-track");
  track.setAttribute("role", "progressbar");
  track.setAttribute("aria-label", `${job.video.title} download`);
  track.setAttribute("aria-valuemin", "0");
  track.setAttribute("aria-valuemax", "100");
  const fill = element("div", "progress-fill");
  track.append(fill);
  const meta = element("div", "progress-meta");
  const percent = element("span");
  const speed = element("span");
  meta.append(percent, speed);
  progress.append(track, meta);
  const error = element("p", "job-error");
  node.append(top, progress, error);
  return {
    node,
    number,
    detail,
    actions,
    progress,
    track,
    fill,
    percent,
    speed,
    error,
    status: null,
  };
}
function renderQueue() {
  const active = queue.jobs.find((j) =>
    ["downloading", "processing"].includes(j.status),
  );
  const pending = queue.jobs.filter((j) => j.status === "queued");
  const finished = queue.jobs.filter((j) =>
    ["completed", "failed", "canceled"].includes(j.status),
  );
  $("queue-count").textContent = pending.length + (active ? 1 : 0);
  $("activity").textContent = active
    ? "Downloading"
    : queue.paused
      ? "Paused"
      : "";
  $("activity").hidden = !active && !queue.paused;
  $("activity").classList.toggle("running", !!active);
  $("pause").hidden = !active && !pending.length;
  $("pause").textContent = queue.paused ? "Resume" : "Pause";
  $("pause").title =
    "Pause holds the next download. The current download keeps running.";
  $("clear-history").hidden = !finished.length;
  $("queue-description").textContent = queue.paused
    ? "Queue paused. Resume when you’re ready."
    : active
      ? `${pending.length} waiting · Downloading one at a time`
      : "Downloads run one at a time.";
  $("empty").hidden = queue.jobs.length > 0;
  $("choose-folder").textContent = queue.folder.replace(
    /^\/Users\/[^/]+\//,
    "~/",
  );
  $("choose-folder").title = `Save to ${queue.folder}. Click to change.`;
  renderPreferences();
  $("footer-status").textContent = active
    ? "Downloading in the background"
    : "";
  if (platform !== "macos" && active)
    $("footer-status").textContent = "Keep the app open while downloading.";
  const ordered = [
    ...(active ? [active] : []),
    ...pending,
    ...finished.slice().reverse(),
  ];
  const ids = new Set(ordered.map((j) => j.id));
  for (const [id, row] of jobNodes) {
    if (!ids.has(id)) {
      row.node.remove();
      jobNodes.delete(id);
    }
  }
  ordered.forEach((job, index) => {
    const isNew = !jobNodes.has(job.id);
    if (isNew) jobNodes.set(job.id, createJobNode(job));
    const row = jobNodes.get(job.id);
    const running = ["downloading", "processing"].includes(job.status);
    row.node.className = `job ${job.status}`;
    row.number.className = `job-number${running ? " active" : ""}`;
    row.number.textContent = running
      ? "↓"
      : job.status === "completed"
        ? "✓"
        : job.status === "failed"
          ? "!"
          : job.status === "canceled"
            ? "−"
            : pending.indexOf(job) + 1;
    row.detail.textContent = `${job.qualityLabel}${!["mp3", "subtitles"].includes(job.format) ? ` · ${(job.container || "mkv").toUpperCase()}${job.video.videoCodec === "av1" ? " · AV1" : ""}` : ""}${job.subtitle ? ` · ${job.subtitle.name}` : ""}${job.status === "queued" ? " · Waiting" : job.status === "completed" ? " · Saved" : job.status === "failed" ? " · Failed" : job.status === "canceled" ? " · Canceled" : ""}`;
    row.progress.hidden = !running;
    row.progress.classList.toggle("processing", job.status === "processing");
    row.fill.style.transform = `scaleX(${job.progress / 100})`;
    row.track.setAttribute("aria-valuenow", Math.round(job.progress));
    row.percent.textContent =
      job.status === "processing"
        ? "Finishing · merging or converting…"
        : `${Math.floor(job.progress)}%`;
    row.speed.textContent = [job.speed, job.eta ? `${job.eta} left` : ""]
      .filter(Boolean)
      .join(" · ");
    row.error.hidden = !job.error;
    row.error.textContent = job.error || "";
    if (row.status !== job.status) {
      row.actions.replaceChildren();
      if (job.status === "queued")
        row.actions.append(
          actionButton("Move earlier", "↑", "up", job.id),
          actionButton("Move later", "↓", "down", job.id),
        );
      if (running || job.status === "queued")
        row.actions.append(
          actionButton("Cancel download", "×", "cancel", job.id),
        );
      if (["failed", "canceled"].includes(job.status))
        row.actions.append(
          actionButton("Retry download", "↻", "retry", job.id),
        );
      if (job.status === "completed") {
        const reveal = element("button", "", "↗");
        reveal.title = "Show in folder";
        reveal.setAttribute("aria-label", "Show in folder");
        reveal.onclick = () =>
          invoke("reveal_folder", { id: job.id }).catch(report);
        row.actions.append(reveal);
      }
      row.status = job.status;
    }
    if ($("queue").children[index] !== row.node)
      $("queue").insertBefore(row.node, $("queue").children[index] || null);
    if (isNew) animateEntry(row.node);
  });
  updateDownloadButton();
}
function renderPreferences() {
  $("settings-folder").textContent = queue.folder.replace(
    /^\/Users\/[^/]+\//,
    "~/",
  );
  $("settings-folder").title = queue.folder;
  if (!savingPreferences) {
    $("default-container").value = queue.videoContainer || "mp4";
    $("default-codec").value = queue.videoCodec || "auto";
  }
}
async function savePreferences() {
  if (savingPreferences) return;
  const codecChanged =
    $("default-codec").value !== (queue.videoCodec || "auto");
  savingPreferences = true;
  $("default-container").disabled = true;
  $("default-codec").disabled = true;
  updateDownloadButton();
  try {
    queue = await invoke("save_preferences", {
      container: $("default-container").value,
      codec: $("default-codec").value,
    });
    if (codecChanged && video) await inspect();
    toast("Download preferences saved.");
  } catch (error) {
    report(error);
  } finally {
    savingPreferences = false;
    $("default-container").disabled = false;
    $("default-codec").disabled = false;
    renderPreferences();
    updateDownloadButton();
  }
}
function showSetup() {
  $("setup").hidden = false;
  $("downloads").hidden = true;
  $("setup-back").hidden = !queue.setupComplete;
  renderPreferences();
}
function showDownloads() {
  $("setup").hidden = true;
  $("downloads").hidden = false;
  renderQueue();
}
function renderDependencies(results) {
  deps = results;
  $("dependencies").replaceChildren();
  deps.forEach((dep) => {
    const row = element("div", "dependency");
    const state = element(
      "span",
      `dependency-state${dep.version ? "" : " missing"}`,
      dep.version ? "✓" : "!",
    );
    const info = element("div", "dependency-info");
    info.append(element("strong", "", dep.name), element("p", "", dep.purpose));
    const shortVersion = dep.version?.match(/\d+\.\d+(?:\.\d+)?/)?.[0];
    const version = element(
      "span",
      "dependency-version",
      dep.version ? shortVersion || "Installed" : "Missing",
    );
    version.title = dep.path
      ? `${dep.version}\n${dep.path}`
      : "Install this tool, then check again.";
    row.append(state, info, version);
    $("dependencies").append(row);
  });
  const missing = deps.filter((d) => d.required && !d.version);
  $("install-help").hidden = !missing.length;
  $("continue").disabled = missing.length > 0;
  $("setup-description").textContent = missing.length
    ? "Restore the missing tools to continue downloading."
    : "All download tools are installed.";
  $("dependency-warning").hidden = !missing.length;
  $("dependency-warning").textContent =
    `Missing ${missing.map((d) => d.name).join(", ")} · Open Settings`;
  $("install-command").textContent =
    platform === "windows"
      ? "winget install --id yt-dlp.yt-dlp -e\nwinget install --id Gyan.FFmpeg -e\nwinget install --id DenoLand.Deno -e"
      : "brew install yt-dlp ffmpeg deno";
}
async function scan() {
  $("rescan").disabled = true;
  $("continue").disabled = true;
  $("dependencies").replaceChildren(
    element("div", "loading", "Checking installed tools…"),
  );
  try {
    renderDependencies(await invoke("scan_dependencies"));
  } catch (error) {
    $("dependencies").textContent = "Could not check dependencies.";
    report(error);
  } finally {
    $("rescan").disabled = false;
  }
}
$("link-form").addEventListener("submit", (event) => {
  event.preventDefault();
  inspect();
});
$("url").addEventListener("input", () => {
  ++request;
  $("metadata-loading").hidden = true;
  $("inspect").disabled = false;
  clearTimeout(pasteTimer);
  resetVideo();
  $("link-hint").textContent = "Choose a video quality, MP3, or subtitles.";
});
$("url").addEventListener("paste", () => {
  clearTimeout(pasteTimer);
  pasteTimer = setTimeout(inspect, 350);
});
$("clear-video").onclick = () => {
  ++request;
  resetVideo();
  $("url").value = "";
  $("url").focus();
};
$("quality").onchange = updateDownloadButton;
$("subtitle").onchange = updateDownloadButton;
$("download").onclick = addDownload;
$("pause").onclick = () => queueAction(queue.paused ? "resume" : "pause");
$("clear-history").onclick = () => queueAction("clear");
async function chooseFolder() {
  try {
    const folder = await invoke("choose_folder");
    if (folder) {
      queue.folder = folder;
      renderQueue();
    }
  } catch (error) {
    report(error);
  }
}
$("choose-folder").onclick = chooseFolder;
$("settings-folder").onclick = chooseFolder;
$("settings-choose-folder").onclick = chooseFolder;
$("default-container").onchange = savePreferences;
$("default-codec").onchange = savePreferences;
$("open-folder").onclick = () =>
  invoke("reveal_folder", { id: null }).catch(report);
$("settings-button").onclick = () => {
  showSetup();
  if (deps.length) renderDependencies(deps);
};
$("dependency-warning").onclick = () => {
  showSetup();
  renderDependencies(deps);
};
$("footer-quit").onclick = () => invoke("quit_app").catch(report);
$("rescan").onclick = scan;
$("continue").onclick = async () => {
  try {
    await invoke("complete_setup");
    queue.setupComplete = true;
    showDownloads();
    $("url").focus();
  } catch (error) {
    report(error);
  }
};
$("setup-back").onclick = showDownloads;
$("copy-command").onclick = () =>
  navigator.clipboard
    .writeText($("install-command").textContent)
    .then(() => toast("Install command copied."))
    .catch(() => toast("Select and copy the command above."));
document.addEventListener("keydown", (event) => {
  if (event.key === "Escape" && platform === "macos")
    invoke("dismiss").catch(report);
});
async function start() {
  await listen("queue-changed", (event) => {
    queue = event.payload;
    renderQueue();
  });
  await listen("app-error", (event) => report(event.payload));
  await listen("dependencies-checked", (event) =>
    renderDependencies(event.payload),
  );
  await listen("panel-open", (event) => {
    // AppKit animates the popover itself. Avoid a second content entrance.
    if (!event.payload?.preserveView && hasOpened && queue.setupComplete)
      showDownloads();
    hasOpened = true;
    if (startupReady && !video && !$("downloads").hidden) $("url").focus();
  });
  const initial = await invoke("bootstrap");
  queue = initial.queue;
  platform = initial.platform;
  $("app-version").textContent = initial.versionLabel;
  document.body.dataset.platform = platform;
  if (initial.dependencies) renderDependencies(initial.dependencies);
  if (initial.firstLaunch) showSetup();
  else showDownloads();
  startupReady = true;
  if (platform === "macos") {
    let lastHeight = 0;
    const resize = () => {
      const height = Math.ceil($("app").getBoundingClientRect().height);
      if (Math.abs(height - lastHeight) > 1) {
        lastHeight = height;
        invoke("resize_panel", { height }).catch(report);
      }
    };
    new ResizeObserver(resize).observe($("app"));
    resize();
  }
}
start().catch(report);
