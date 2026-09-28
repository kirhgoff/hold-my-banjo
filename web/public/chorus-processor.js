class ChorusProcessor extends AudioWorkletProcessor {
  constructor({ processorOptions: { module, params } }) {
    super();
    this.wasm = new WebAssembly.Instance(module, {}).exports;
    this.engine = 0;
    this.create(params);
    this.port.onmessage = ({ data }) => {
      if (data.type === 'reset') this.create(data.params);
      if (data.type === 'set') this.set(data.key, data.value);
      if (data.type === 'bonk') this.setBonk(data.params);
      if (data.type === 'audition') this.wasm.hmb_audition(this.engine);
    };
  }

  create(p) {
    if (this.engine) this.wasm.hmb_free(this.engine);
    this.engine = this.wasm.hmb_new(sampleRate, p.seed, p.frogs, p.mode, p.strength, p.interval, p.radius);
    this.setBonk(p.bonk);
    this.postFrogs();
  }

  set(key, value) {
    const e = this.engine, w = this.wasm;
    if (key === 'frogs') { w.hmb_set_frog_count(e, value); this.postFrogs(); }
    else if (key === 'mode') w.hmb_set_mode(e, value);
    else if (key === 'strength') w.hmb_set_strength(e, value);
    else if (key === 'interval') w.hmb_set_mean_interval(e, value);
    else if (key === 'radius') w.hmb_set_hearing_radius(e, value);
  }

  setBonk(b) {
    const e = this.engine, w = this.wasm;
    w.hmb_set_bonk(e, ...b);
    const samples = new Float32Array(w.memory.buffer, w.hmb_bonk_samples(e), w.hmb_bonk_len(e)).slice();
    this.port.postMessage({ type: 'bonk-shape', samples });
  }

  postFrogs() {
    const n = this.wasm.hmb_frog_count(this.engine);
    const positions = Array.from(new Float64Array(this.wasm.memory.buffer, this.wasm.hmb_positions(this.engine), n * 2));
    this.port.postMessage({ type: 'frogs', positions, width: this.wasm.hmb_scene_width(this.engine), depth: this.wasm.hmb_scene_depth(this.engine) });
  }

  process(_inputs, outputs) {
    const [left, right] = outputs[0];
    const n = left.length;
    const ptr = this.wasm.hmb_render(this.engine, n);
    const buf = new Float32Array(this.wasm.memory.buffer, ptr, n * 2);
    for (let i = 0; i < n; i++) { left[i] = buf[2 * i]; right[i] = buf[2 * i + 1]; }
    const count = this.wasm.hmb_event_count(this.engine);
    if (count) this.port.postMessage({ type: 'calls', ids: Array.from(new Uint32Array(this.wasm.memory.buffer, this.wasm.hmb_events(this.engine), count)) });
    return true;
  }
}

registerProcessor('chorus', ChorusProcessor);
