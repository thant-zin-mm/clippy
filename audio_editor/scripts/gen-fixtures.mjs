#!/usr/bin/env node
/**
 * Generate tiny WAV fixtures with known silence spans (mono, 16-bit PCM).
 *
 * Usage: node scripts/gen-fixtures.mjs [outDir]
 * Default outDir: fixtures/ (gitignored; regenerate as needed).
 *
 * The default "pattern" fixture contains two silences:
 *   1.50s – 3.00s  (1.5 s)
 *   4.00s – 7.00s  (3.0 s)
 * MP3 fixtures are generated later (P1-T1) once the lame encoder is wired.
 */
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const SAMPLE_RATE = 44_100;

/** seconds, amplitude; amp 0 = digital silence */
const SEGMENTS = [
  { seconds: 1.5, amp: 0.3, freq: 440 },
  { seconds: 1.5, amp: 0 },
  { seconds: 1.0, amp: 0.3, freq: 220 },
  { seconds: 3.0, amp: 0 },
  { seconds: 2.0, amp: 0.25, freq: 330 },
];

function encodeWav(samples) {
  const dataSize = samples.length * 2;
  const buffer = Buffer.alloc(44 + dataSize);
  buffer.write('RIFF', 0);
  buffer.writeUInt32LE(36 + dataSize, 4);
  buffer.write('WAVE', 8);
  buffer.write('fmt ', 12);
  buffer.writeUInt32LE(16, 16); // PCM chunk size
  buffer.writeUInt16LE(1, 20); // format = PCM
  buffer.writeUInt16LE(1, 22); // channels = mono
  buffer.writeUInt32LE(SAMPLE_RATE, 24);
  buffer.writeUInt32LE(SAMPLE_RATE * 2, 28); // byte rate
  buffer.writeUInt16LE(2, 32); // block align
  buffer.writeUInt16LE(16, 34); // bits per sample
  buffer.write('data', 36);
  buffer.writeUInt32LE(dataSize, 40);
  for (let i = 0; i < samples.length; i += 1) {
    const clamped = Math.max(-1, Math.min(1, samples[i]));
    buffer.writeInt16LE(Math.round(clamped * 32_767), 44 + i * 2);
  }
  return buffer;
}

function synthesize(segments) {
  const samples = [];
  for (const segment of segments) {
    const count = Math.round(segment.seconds * SAMPLE_RATE);
    for (let i = 0; i < count; i += 1) {
      if (segment.amp === 0) {
        samples.push(0);
      } else {
        samples.push(segment.amp * Math.sin((2 * Math.PI * segment.freq * i) / SAMPLE_RATE));
      }
    }
  }
  return samples;
}

const outDir = process.argv[2] ?? 'fixtures';
mkdirSync(outDir, { recursive: true });
const outFile = join(outDir, 'pattern.wav');
writeFileSync(outFile, encodeWav(synthesize(SEGMENTS)));
const duration = SEGMENTS.reduce((sum, s) => sum + s.seconds, 0);
console.log(`wrote ${outFile} (${duration.toFixed(1)}s @ ${SAMPLE_RATE} Hz mono)`);
