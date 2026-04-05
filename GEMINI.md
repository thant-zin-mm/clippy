# Clippy - Audio Player & Trimmer

Clippy is a lightweight client-side application designed to play and trim audio files. It's built with a mobile-first, mobile-exclusive UI that fits perfectly on a mobile screen, even when viewed on a desktop.

## Key Features
- **Audio Playback**: Precise waveform-based playback with independent Start and End preview toggles.
- **Precision Trimming**: High-accuracy inputs (HH:MM:SS:mmm) with +/- 1s and +/- 100ms adjustment buttons.
- **Waveform Visualization**: Real-time selection highlighting with blue regions and slate gray position markers.
- **Smart Tools**: "Set as Start" and "Set as End" buttons to instantly capture timestamps from the playhead.
- **Advanced Loading**: Smooth progress bar with file size display and heuristic-based estimated loading time.
- **Dark/Light Mode**: Toggle between dark and light themes with persistent settings and theme-aware waveform colors.
- **Input Guardrails**: Masked inputs for timestamps with real-time visual validation (red highlighting).
- **Mobile-Exclusive UI**: Optimized for mobile devices, centered on desktop with a professional application-like navbar.

## Tech Stack
- **Framework**: Vite
- **UI Framework**: Tailwind CSS v4 (Base styles + custom CSS)
- **Language**: TypeScript
- **Audio Engine**: wavesurfer.js (with Regions plugin)
- **Export**: Client-side WAV encoding

## Setup & Development
1. `npm install` - Install dependencies.
2. `npm run dev` - Start the Vite dev server.

## Progress Tracking
- [x] Initialize project with Vite & TypeScript
- [x] Install and configure Tailwind CSS v4
- [x] Implement mobile-exclusive layout (centered on desktop)
- [x] Add Navbar with Dark/Light mode toggle (persistent)
- [x] Implement Audio Upload with folder-open icon in Navbar
- [x] Implement Waveform visualization with wavesurfer.js
- [x] Add Precision Audio Trimming (Selection and Export)
- [x] Implement specialized Time Input Masks (HH:MM:SS:mmm)
- [x] Add independent Play/Pause controls for Start/End previews
- [x] Implement "Set as Start/End" from current playhead position
- [x] Add "Audio Track Length" display and visual validation (red cues)
- [x] Implement advanced Loading UI with progress bar and estimates
- [x] Apply professional skyblue/cyan accented theme across all components

## TODO
- [ ] Implement multi-segment selection?
- [ ] Add audio effects (Fade in/out)?
- [ ] Support more export formats (MP3 encoding via library)?
