# Clippy - Audio Player & Trimmer

Clippy is a lightweight client-side application designed to play and trim audio files. It's built with a mobile-first, mobile-exclusive UI that fits perfectly on a mobile screen, even when viewed on a desktop.

## Key Features
- **Dashboard UI**: Clean, professional card-based interface with a unified Info Bar for position and duration.
- **Independent Previews**: Dedicated Start and End point previews. The End Point preview smartly plays the last 2 seconds of your selection.
- **Precision Trimming**: Masked time inputs (HH:MM:SS:mmm) with icon-based coarse (<< / >>) and fine-tuning (< / >) steppers.
- **Capture Tools**: One-click "Thumbtack" icons to instantly pin the current playhead position to Start or End points.
- **Waveform Selection**: Visual blue region highlighting with real-time synchronization to inputs.
- **Advanced Loading**: Heuristic-based loading estimates with file size and format detection.
- **User Guide**: Integrated visual guide explaining each component and tool.
- **Input Guardrails**: Real-time visual validation (red highlighting) for out-of-bounds or invalid time ranges.

## Tech Stack
- **Framework**: Vite
- **UI Framework**: Tailwind CSS v4 (Base) + Optimized Custom CSS
- **Language**: TypeScript
- **Audio Engine**: wavesurfer.js (Regions Plugin)
- **Export**: Client-side WAV encoding

## Setup & Development
1. `npm install` - Install dependencies.
2. `npm run dev` - Start the Vite dev server.

## Progress Tracking
- [x] Initialize project with Vite & TypeScript
- [x] Install and configure Tailwind CSS v4
- [x] Implement mobile-exclusive layout (centered on desktop)
- [x] Add Navbar with Integrated Actions (File, Help, Theme)
- [x] Implement Waveform visualization with Regions plugin
- [x] Create Unified Info Bar (Position/Length)
- [x] Design Control Card system for Start/End points
- [x] Implement icon-based precision steppers
- [x] Add Thumbtack "Capture" functionality
- [x] Implement independent, smart preview logic
- [x] Add visual input validation (red status cues)
- [x] Implement scrollable User Guide modal
- [x] Apply professional skyblue/cyan dashboard aesthetic

## TODO
- [ ] Implement multi-segment selection?
- [ ] Add audio effects (Fade in/out)?
- [ ] Support more export formats (MP3 encoding via library)?
