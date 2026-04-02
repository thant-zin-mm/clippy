# Clippy - Audio Player & Trimmer

Clippy is a lightweight client-side application designed to play and trim audio files. It's built with a mobile-first, mobile-exclusive UI that fits perfectly on a mobile screen, even when viewed on a desktop.

## Key Features
- **Audio Playback**: Waveform-based playback with a play/pause toggle.
- **Audio Trimming**: (Planned) Precision trimming of audio files.
- **Dark/Light Mode**: Toggle between dark and light themes with persistent settings.
- **Mobile-Exclusive UI**: Optimized for mobile devices, centered on desktop with a professional application-like toolbar.
- **Accented Theme**: Professional and elegant skyblue/cyan accented interface.

## Tech Stack
- **Framework**: Vite
- **UI Framework**: Tailwind CSS v4
- **Language**: TypeScript
- **Styling**: CSS with @tailwindcss/postcss
- **Audio Engine**: wavesurfer.js

## Setup & Development
1. `npm install` - Install dependencies.
2. `npm run dev` - Start the Vite dev server.

## Progress Tracking
- [x] Initialize project with Vite & TypeScript
- [x] Install and configure Tailwind CSS v4
- [x] Implement mobile-exclusive layout (centered on desktop)
- [x] Add Navbar with Dark/Light mode toggle (persistent)
- [x] Implement Audio Upload via Top Toolbar
- [x] Implement Waveform visualization with wavesurfer.js
- [x] Add Play/Pause audio controls
- [x] Implement "Loading please wait..." UI
- [x] Apply professional skyblue/cyan accented theme
- [ ] Implement Audio Trimming (Selection and Export)
