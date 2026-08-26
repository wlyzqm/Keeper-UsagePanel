// Display-only smoothing. Keeper samples and detail metrics stay unmodified.
export class WidgetDeltaDisplay {
  constructor(idleSeconds = 16) {
    this.idleSeconds = idleSeconds;
    this.clear();
  }
  clear() {
    this.lastSample = null;
    this.zeroSeconds = 0;
    this.value = { input: null, output: null, held: false };
    return this.value;
  }
  update(sample, disconnected = false) {
    if (
      disconnected ||
      !sample ||
      sample.delta.baseline ||
      sample.delta.reset
    ) {
      return this.clear();
    }
    // A poll is delivered both as an event and as the invoke result.
    if (sample.sampled_at === this.lastSample) return this.value;
    this.lastSample = sample.sampled_at;
    const {
      input_tokens: input,
      output_tokens: output,
      seconds,
    } = sample.delta;
    if (input !== 0 || output !== 0) {
      this.zeroSeconds = 0;
      this.value = { input, output, held: false };
    } else {
      this.zeroSeconds += seconds;
      const hasPrevious = this.value.input > 0 || this.value.output > 0;
      this.value =
        hasPrevious && this.zeroSeconds < this.idleSeconds
          ? { ...this.value, held: true }
          : { input: 0, output: 0, held: false };
    }
    return this.value;
  }
}
