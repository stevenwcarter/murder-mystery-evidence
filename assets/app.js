// Live round updates and the pinch-zoom evidence viewer. No dependencies.
"use strict";

// Live update: reload when the detective moves to another round.
const live = document.getElementById("live");
const setLive = (state) => {
  live.textContent = state;
  live.dataset.state = state;
};
const events = new EventSource("/events");
events.addEventListener("open", () => setLive("live"));
events.addEventListener("error", () => setLive("reconnecting"));
events.addEventListener("round", (event) => {
  if (event.data !== document.body.dataset.round) location.reload();
});

// Viewer.
const viewer = document.getElementById("viewer");
const stage = viewer.querySelector(".stage");
const image = document.getElementById("viewer-image");
const caption = document.getElementById("viewer-title");
const download = document.getElementById("viewer-download");
const openLink = document.getElementById("viewer-open");

const MIN_SCALE = 1;
const MAX_SCALE = 6;
const TAP_SCALE = 2.5;
const DOUBLE_TAP_MS = 300;

let scale = MIN_SCALE;
let x = 0;
let y = 0;
let lastTap = 0;
const pointers = new Map();

const clamp = (value, low, high) => Math.min(high, Math.max(low, value));
const render = () => {
  image.style.transform = `translate(${x}px, ${y}px) scale(${scale})`;
};
const reset = () => {
  scale = MIN_SCALE;
  x = 0;
  y = 0;
  lastTap = 0;
  pointers.clear();
  render();
};

// Scales to `next` keeping the image point under (px, py) still. The image's
// transform origin is its top-left corner, so its untransformed position is
// the on-screen position minus the current translation.
const zoomAt = (px, py, next) => {
  next = clamp(next, MIN_SCALE, MAX_SCALE);
  const rect = image.getBoundingClientRect();
  const qx = px - (rect.left - x);
  const qy = py - (rect.top - y);
  x = qx - (next / scale) * (qx - x);
  y = qy - (next / scale) * (qy - y);
  scale = next;
  if (scale === MIN_SCALE) {
    x = 0;
    y = 0;
  }
  render();
};

const pinchOf = () => {
  const [a, b] = pointers.values();
  return {
    mx: (a.x + b.x) / 2,
    my: (a.y + b.y) / 2,
    distance: Math.hypot(a.x - b.x, a.y - b.y),
  };
};

stage.addEventListener("pointerdown", (event) => {
  stage.setPointerCapture(event.pointerId);
  pointers.set(event.pointerId, { x: event.clientX, y: event.clientY });
  if (pointers.size !== 1) return;
  if (event.timeStamp - lastTap < DOUBLE_TAP_MS) {
    zoomAt(event.clientX, event.clientY, scale > MIN_SCALE ? MIN_SCALE : TAP_SCALE);
    lastTap = 0;
  } else {
    lastTap = event.timeStamp;
  }
});

stage.addEventListener("pointermove", (event) => {
  const previous = pointers.get(event.pointerId);
  if (!previous) return;
  const current = { x: event.clientX, y: event.clientY };
  if (pointers.size === 2) {
    const before = pinchOf();
    pointers.set(event.pointerId, current);
    const after = pinchOf();
    x += after.mx - before.mx;
    y += after.my - before.my;
    if (before.distance > 0) {
      zoomAt(after.mx, after.my, (scale * after.distance) / before.distance);
    }
    return;
  }
  pointers.set(event.pointerId, current);
  if (pointers.size === 1 && scale > MIN_SCALE) {
    x += current.x - previous.x;
    y += current.y - previous.y;
    render();
  }
});

const release = (event) => pointers.delete(event.pointerId);
stage.addEventListener("pointerup", release);
stage.addEventListener("pointercancel", release);

// A tap on the black around the image closes the viewer, unless zoomed in.
// Pointer capture retargets the click to the stage, so test the position.
stage.addEventListener("click", (event) => {
  const rect = image.getBoundingClientRect();
  const outside =
    event.clientX < rect.left ||
    event.clientX > rect.right ||
    event.clientY < rect.top ||
    event.clientY > rect.bottom;
  if (outside && scale === MIN_SCALE) viewer.close();
});
document.getElementById("viewer-close").addEventListener("click", () => viewer.close());
viewer.addEventListener("close", reset);

document.addEventListener("click", (event) => {
  const link = event.target.closest("a.view");
  if (!link || event.metaKey || event.ctrlKey || event.shiftKey) return;
  event.preventDefault();
  image.src = link.href;
  image.alt = link.dataset.title;
  caption.textContent = link.dataset.title;
  download.href = link.href;
  openLink.href = link.href;
  reset();
  viewer.showModal();
});
