// RustView Mockup — Interaction Scripts

// ── State ──────────────────────────────────────────────────────────────────
const state = {
  view: 'viewer',      // 'viewer' | 'gallery'
  infoOpen: true,
  zoom: 100,
  imageIndex: 3,
  totalImages: 24,
};

// Mock image list (unsplash placeholders)
const IMAGES = [
  { name: 'mountain_sunrise.jpg', w: 4032, h: 3024, size: '4.2 MB', date: '2025-08-14', format: 'JPEG', src: 'https://picsum.photos/seed/a1/800/600' },
  { name: 'city_lights.png',      w: 5472, h: 3648, size: '8.1 MB', date: '2025-07-22', format: 'PNG',  src: 'https://picsum.photos/seed/a2/800/600' },
  { name: 'forest_path.jpg',      w: 3840, h: 2560, size: '3.7 MB', date: '2025-06-10', format: 'JPEG', src: 'https://picsum.photos/seed/a3/800/600' },
  { name: 'ocean_wave.webp',      w: 6016, h: 4016, size: '2.1 MB', date: '2025-05-05', format: 'WebP', src: 'https://picsum.photos/seed/a4/800/600' },
  { name: 'desert_dune.jpg',      w: 4912, h: 3264, size: '5.3 MB', date: '2025-04-18', format: 'JPEG', src: 'https://picsum.photos/seed/a5/800/600' },
  { name: 'snow_peak.png',        w: 2560, h: 1440, size: '6.9 MB', date: '2025-03-02', format: 'PNG',  src: 'https://picsum.photos/seed/a6/800/600' },
];

// ── Helpers ────────────────────────────────────────────────────────────────
const $ = id => document.getElementById(id);
const $$ = sel => document.querySelectorAll(sel);

function clamp(val, min, max) { return Math.max(min, Math.min(max, val)); }

// ── Zoom ───────────────────────────────────────────────────────────────────
function setZoom(z) {
  state.zoom = clamp(z, 10, 1600);
  const zoomValEl = document.querySelector('.zoom-val');
  if (zoomValEl) zoomValEl.textContent = state.zoom + '%';
  const img = document.querySelector('.viewer-image');
  if (img) img.style.transform = `scale(${state.zoom / 100})`;
  const badge = document.querySelector('.zoom-badge');
  if (badge) badge.textContent = state.zoom + '%';
  updateStatusBar();
}

function zoomIn()  { setZoom(Math.round(state.zoom * 1.25)); }
function zoomOut() { setZoom(Math.round(state.zoom / 1.25)); }
function zoomFit() { setZoom(100); }
function zoom1x()  { setZoom(100); }

// ── Navigation ─────────────────────────────────────────────────────────────
function navigate(delta) {
  state.imageIndex = ((state.imageIndex + delta) + IMAGES.length) % IMAGES.length;
  loadImage(state.imageIndex);
}

function loadImage(idx) {
  const img = IMAGES[idx % IMAGES.length];
  // Update viewer image src
  const viewerImg = document.querySelector('.viewer-image');
  if (viewerImg) {
    viewerImg.style.opacity = '0';
    viewerImg.src = img.src + '?t=' + idx;
    viewerImg.onload = () => { viewerImg.style.opacity = '1'; };
  }
  // Update toolbar title
  const titleEl = document.querySelector('.toolbar-title');
  if (titleEl) titleEl.innerHTML = `<span>${img.name}</span>`;
  // Update info panel
  updateInfoPanel(img);
  // Update status
  updateStatusBar();
  // Update gallery selection
  $$('.gallery-item').forEach((el, i) => {
    el.classList.toggle('selected', i === idx);
  });
  state.zoom = 100;
  setZoom(100);
}

// ── Info Panel ─────────────────────────────────────────────────────────────
function toggleInfo() {
  state.infoOpen = !state.infoOpen;
  const panel = document.querySelector('.info-panel');
  if (panel) panel.classList.toggle('hidden', !state.infoOpen);
  const btn = $('btn-info');
  if (btn) btn.classList.toggle('active', state.infoOpen);
}

function updateInfoPanel(img) {
  const rows = {
    'info-filename': img.name,
    'info-format':   img.format,
    'info-size':     img.size,
    'info-dims':     `${img.w} × ${img.h}`,
    'info-date':     img.date,
  };
  for (const [id, val] of Object.entries(rows)) {
    const el = $(id);
    if (el) el.textContent = val;
  }
  const thumb = document.querySelector('.info-thumb');
  if (thumb) thumb.src = img.src;
}

// ── Status Bar ─────────────────────────────────────────────────────────────
function updateStatusBar() {
  const img = IMAGES[state.imageIndex % IMAGES.length];
  const idxEl = $('status-index');
  if (idxEl) idxEl.textContent = `${state.imageIndex + 1} / ${IMAGES.length}`;
  const dimsEl = $('status-dims');
  if (dimsEl) dimsEl.textContent = `${img.w} × ${img.h}`;
  const zEl = $('status-zoom');
  if (zEl) zEl.textContent = state.zoom + '%';
}

// ── View Toggle ────────────────────────────────────────────────────────────
function switchView(mode) {
  state.view = mode;
  const viewer  = $('view-viewer');
  const gallery = $('view-gallery');
  if (viewer)  viewer.style.display  = mode === 'viewer'  ? 'flex' : 'none';
  if (gallery) gallery.style.display = mode === 'gallery' ? 'flex' : 'none';
  $$('.view-tab').forEach(t => t.classList.toggle('active', t.dataset.view === mode));
}

// ── Keyboard ───────────────────────────────────────────────────────────────
document.addEventListener('keydown', e => {
  if (e.key === 'ArrowLeft'  || e.key === 'ArrowUp')   navigate(-1);
  if (e.key === 'ArrowRight' || e.key === 'ArrowDown')  navigate(+1);
  if (e.key === '+' || e.key === '=')  zoomIn();
  if (e.key === '-')                   zoomOut();
  if (e.key === '0')                   zoomFit();
  if (e.key === 'i' || e.key === 'I')  toggleInfo();
  if (e.key === 'g' || e.key === 'G')  switchView(state.view === 'viewer' ? 'gallery' : 'viewer');
  if (e.key === 'f' || e.key === 'F')  toggleFullscreen();
});

// ── Mouse Wheel Zoom ───────────────────────────────────────────────────────
document.querySelector && document.addEventListener('wheel', e => {
  if (state.view !== 'viewer') return;
  e.preventDefault();
  if (e.deltaY < 0) zoomIn(); else zoomOut();
}, { passive: false });

// ── Fullscreen ─────────────────────────────────────────────────────────────
function toggleFullscreen() {
  if (!document.fullscreenElement) document.documentElement.requestFullscreen?.();
  else document.exitFullscreen?.();
}

// ── Init ───────────────────────────────────────────────────────────────────
document.addEventListener('DOMContentLoaded', () => {
  loadImage(state.imageIndex);
  updateStatusBar();
  switchView('viewer');
});
