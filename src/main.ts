import './style.css'
import WaveSurfer from 'wavesurfer.js'

/**
 * Handles the dark/light mode toggle.
 */
function setupThemeToggle(onThemeChange: (theme: 'dark' | 'light') => void) {
  const themeToggle = document.getElementById('theme-toggle')
  const themeIcon = document.getElementById('theme-icon')
  const htmlElement = document.documentElement
  const subtext = document.getElementById('subtext')

  // Check for saved theme in localStorage
  const savedTheme = (localStorage.getItem('theme') as 'dark' | 'light') || 'light'
  
  if (savedTheme === 'dark') {
    htmlElement.classList.add('dark')
    updateUI('dark')
    onThemeChange('dark')
  } else {
    htmlElement.classList.remove('dark')
    updateUI('light')
    onThemeChange('light')
  }

  themeToggle?.addEventListener('click', () => {
    const isDark = htmlElement.classList.toggle('dark')
    const nextTheme = isDark ? 'dark' : 'light'

    localStorage.setItem('theme', nextTheme)
    updateUI(nextTheme)
    onThemeChange(nextTheme)
  })

  function updateUI(theme: string) {
    if (themeIcon) {
      if (theme === 'dark') {
        themeIcon.classList.remove('fa-moon')
        themeIcon.classList.add('fa-sun')
        themeIcon.style.color = '#facc15'
      } else {
        themeIcon.classList.remove('fa-sun')
        themeIcon.classList.add('fa-moon')
        themeIcon.style.color = 'inherit'
      }
    }

    if (subtext) {
      subtext.style.color = theme === 'dark' ? '#94a3b8' : '#64748b'
    }
  }
}

/**
 * Handles audio upload, waveform visualization, and playback.
 */
function setupAudioApp() {
  const uploadBtn = document.getElementById('upload-btn')
  const audioInput = document.getElementById('audio-input') as HTMLInputElement
  const welcomeMessage = document.getElementById('welcome-message')
  const loadingContainer = document.getElementById('loading-container')
  const progressBarFill = document.getElementById('progress-bar-fill')
  const waveformContainer = document.getElementById('waveform-container')
  const timeDisplay = document.getElementById('waveform-time')
  const playPauseBtn = document.getElementById('play-pause-btn')
  const playPauseIcon = document.getElementById('play-pause-icon')

  let wavesurfer: WaveSurfer | null = null

  const getThemeColors = (theme: 'dark' | 'light') => {
    return theme === 'dark' 
      ? { wave: '#475569', progress: '#3b82f6' }
      : { wave: '#cbd5e1', progress: '#3b82f6' }
  }

  const currentTheme = () => document.documentElement.classList.contains('dark') ? 'dark' : 'light'

  const initWaveSurfer = () => {
    const colors = getThemeColors(currentTheme())
    
    wavesurfer = WaveSurfer.create({
      container: '#waveform',
      waveColor: colors.wave,
      progressColor: colors.progress,
      cursorColor: '#3b82f6',
      height: 128,
    })

    // Loading progress
    wavesurfer.on('loading', (percent) => {
      if (progressBarFill) {
        progressBarFill.style.width = `${percent}%`
      }
    })

    wavesurfer.on('ready', () => {
      if (loadingContainer) loadingContainer.style.display = 'none'
      if (waveformContainer) waveformContainer.style.display = 'block'
      
      const duration = wavesurfer?.getDuration() || 0
      if (timeDisplay) {
        timeDisplay.textContent = `0:00 / ${formatTime(duration)}`
      }
    })

    wavesurfer.on('play', () => {
      if (playPauseIcon) {
        playPauseIcon.classList.remove('fa-play')
        playPauseIcon.classList.add('fa-pause')
      }
    })

    wavesurfer.on('pause', () => {
      if (playPauseIcon) {
        playPauseIcon.classList.remove('fa-pause')
        playPauseIcon.classList.add('fa-play')
      }
    })

    wavesurfer.on('timeupdate', (currentTime) => {
      if (timeDisplay && wavesurfer) {
        const duration = wavesurfer.getDuration()
        timeDisplay.textContent = `${formatTime(currentTime)} / ${formatTime(duration)}`
      }
    })

    wavesurfer.on('finish', () => {
      if (playPauseIcon) {
        playPauseIcon.classList.remove('fa-pause')
        playPauseIcon.classList.add('fa-play')
      }
    })
  }

  const formatTime = (seconds: number) => {
    const minutes = Math.floor(seconds / 60)
    const secs = Math.floor(seconds % 60)
    return `${minutes}:${secs.toString().padStart(2, '0')}`
  }

  uploadBtn?.addEventListener('click', () => {
    audioInput?.click()
  })

  audioInput?.addEventListener('change', (event) => {
    const file = (event.target as HTMLInputElement).files?.[0]
    if (file) {
      if (!wavesurfer) {
        initWaveSurfer()
      }

      // Prepare UI for loading
      if (welcomeMessage) welcomeMessage.style.display = 'none'
      if (waveformContainer) waveformContainer.style.display = 'none'
      if (loadingContainer) {
        loadingContainer.style.display = 'block'
        if (progressBarFill) progressBarFill.style.width = '0%'
      }

      const url = URL.createObjectURL(file)
      wavesurfer?.load(url)
    }
  })

  playPauseBtn?.addEventListener('click', () => {
    wavesurfer?.playPause()
  })

  // Theme change listener for WaveSurfer
  setupThemeToggle((theme) => {
    if (wavesurfer) {
      const colors = getThemeColors(theme)
      wavesurfer.setOptions({
        waveColor: colors.wave,
        progressColor: colors.progress,
      })
    }
  })
}

// Initialize the app
if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', setupAudioApp)
} else {
  setupAudioApp()
}
