const frogsInput = document.getElementById('frogs') as HTMLInputElement;
const seedInput = document.getElementById('seed') as HTMLInputElement;
const modeInput = document.getElementById('mode') as HTMLSelectElement;
const strengthInput = document.getElementById('strength') as HTMLInputElement;
const intervalInput = document.getElementById('interval') as HTMLInputElement;
const radiusInput = document.getElementById('radius') as HTMLInputElement;
const playButton = document.getElementById('play') as HTMLButtonElement;
const sceneCanvas = document.getElementById('scene') as HTMLCanvasElement;
const waveCanvas = document.getElementById('wave') as HTMLCanvasElement;

const sceneCtx = sceneCanvas.getContext('2d')!;
const waveCtx = waveCanvas.getContext('2d')!;

let ctx: AudioContext | null = null;
let node: AudioWorkletNode | null = null;
let analyser: AnalyserNode | null = null;
let wasmModule: WebAssembly.Module | null = null;

let scenePositions: number[] = [];
let sceneWidth = 30;
let sceneDepth = 20;
const lastCall = new Map<number, number>();

function params() {
  return {
    frogs: Number(frogsInput.value),
    seed: Number(seedInput.value),
    mode: Number(modeInput.value),
    strength: Number(strengthInput.value),
    interval: Number(intervalInput.value),
    radius: Number(radiusInput.value),
  };
}

function updateOutput(input: HTMLInputElement) {
  const out = document.getElementById(`${input.id}-out`);
  if (out) out.textContent = input.value;
}

async function start() {
  ctx = new AudioContext();
  if (!wasmModule) {
    wasmModule = await WebAssembly.compileStreaming(fetch('/chorus.wasm'));
  }
  await ctx.audioWorklet.addModule('/chorus-processor.js');
  node = new AudioWorkletNode(ctx, 'chorus', {
    numberOfInputs: 0,
    outputChannelCount: [2],
    processorOptions: { module: wasmModule, params: params() },
  });
  analyser = ctx.createAnalyser();
  analyser.fftSize = 2048;
  node.connect(analyser).connect(ctx.destination);
  node.port.onmessage = ({ data }) => {
    if (data.type === 'frogs') {
      scenePositions = data.positions;
      sceneWidth = data.width;
      sceneDepth = data.depth;
    } else if (data.type === 'calls') {
      const now = performance.now();
      for (const id of data.ids as number[]) lastCall.set(id, now);
    }
  };
  requestAnimationFrame(draw);
}

async function togglePlay() {
  if (!ctx) {
    playButton.disabled = true;
    await start();
    playButton.disabled = false;
    playButton.textContent = 'Stop';
    return;
  }
  if (ctx.state === 'running') {
    await ctx.suspend();
    playButton.textContent = 'Play';
  } else {
    await ctx.resume();
    playButton.textContent = 'Stop';
  }
}

function drawScene() {
  const w = sceneCanvas.width;
  const h = sceneCanvas.height;
  sceneCtx.clearRect(0, 0, w, h);

  const toCanvasX = (x: number) => ((x + sceneWidth / 2) / sceneWidth) * w;
  const toCanvasY = (y: number) => h - (y / sceneDepth) * h;

  sceneCtx.fillStyle = '#d6e4d0';
  sceneCtx.beginPath();
  sceneCtx.arc(toCanvasX(0), h, 6, Math.PI, 0);
  sceneCtx.fill();

  const now = performance.now();
  for (let i = 0; i < scenePositions.length / 2; i++) {
    const x = scenePositions[2 * i];
    const y = scenePositions[2 * i + 1];
    const last = lastCall.get(i) ?? -Infinity;
    const alpha = 0.3 + 0.7 * Math.max(0, 1 - (now - last) / 400);
    sceneCtx.fillStyle = `rgba(224, 184, 74, ${alpha})`;
    sceneCtx.beginPath();
    sceneCtx.arc(toCanvasX(x), toCanvasY(y), 7, 0, Math.PI * 2);
    sceneCtx.fill();
  }
}

function drawWave() {
  const w = waveCanvas.width;
  const h = waveCanvas.height;
  waveCtx.clearRect(0, 0, w, h);
  if (!analyser) return;

  const buf = new Float32Array(analyser.fftSize);
  analyser.getFloatTimeDomainData(buf);

  waveCtx.strokeStyle = '#e0b84a';
  waveCtx.lineWidth = 1.5;
  waveCtx.beginPath();
  for (let i = 0; i < buf.length; i++) {
    const x = (i / (buf.length - 1)) * w;
    const y = h / 2 - buf[i] * (h / 2);
    if (i === 0) waveCtx.moveTo(x, y);
    else waveCtx.lineTo(x, y);
  }
  waveCtx.stroke();
}

function draw() {
  drawScene();
  drawWave();
  requestAnimationFrame(draw);
}

function bindControl(input: HTMLInputElement | HTMLSelectElement, key: string) {
  input.addEventListener('input', () => {
    if (input instanceof HTMLInputElement) updateOutput(input);
    node?.port.postMessage({ type: 'set', key, value: Number(input.value) });
  });
}

export function init() {
  playButton.addEventListener('click', () => {
    togglePlay();
  });

  bindControl(frogsInput, 'frogs');
  bindControl(modeInput, 'mode');
  bindControl(strengthInput, 'strength');
  bindControl(intervalInput, 'interval');
  bindControl(radiusInput, 'radius');

  seedInput.addEventListener('change', () => {
    node?.port.postMessage({ type: 'reset', params: params() });
  });
}
