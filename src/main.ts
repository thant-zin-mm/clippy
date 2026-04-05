import './style.css'
import WaveSurfer from 'wavesurfer.js'
import RegionsPlugin from 'wavesurfer.js/dist/plugins/regions.esm.js'

/**
 * Handles the dark/light mode toggle.
 */
function setupThemeToggle(onThemeChange: (theme: 'dark' | 'light') => void) {
  const themeToggle = document.getElementById('theme-toggle')
  const themeIcon = document.getElementById('theme-icon')
  const htmlElement = document.documentElement

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
  }
}

/**
 * Utility functions for time formatting and parsing.
 */
function formatTimeWithMs(seconds: number): string {
  const h = Math.floor(seconds / 3600)
  const m = Math.floor((seconds % 3600) / 60)
  const s = Math.floor(seconds % 60)
  const ms = Math.floor((seconds % 1) * 1000)
  
  return [
    h.toString().padStart(2, '0'),
    m.toString().padStart(2, '0'),
    s.toString().padStart(2, '0'),
    ms.toString().padStart(3, '0')
  ].join(':')
}

function parseTime(timeString: string): number | null {
  const parts = timeString.split(':').map(Number)
  if (parts.length !== 4 || parts.some(isNaN)) return null
  
  const [h, m, s, ms] = parts
  return h * 3600 + m * 60 + s + ms / 1000
}

function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 Bytes'
  const k = 1024
  const sizes = ['Bytes', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i]
}

/**
 * Input Mask Logic for HH:MM:SS:mmm with Manual Validation
 */
function setupTimeInputMask(input: HTMLInputElement, onValueChange: (isManual: boolean) => void) {
  const format = '00:00:00:000'
  
  input.addEventListener('keydown', (e) => {
    if (e.key === 'Tab' || e.key === 'ArrowLeft' || e.key === 'ArrowRight' || e.key === 'Home' || e.key === 'End') {
      return
    }

    e.preventDefault()

    const start = input.selectionStart ?? 0
    let value = input.value

    if (/^\d$/.test(e.key)) {
      let pos = start
      if (pos === 2 || pos === 5 || pos === 8) pos++
      
      if (pos < format.length) {
        const charArray = value.split('')
        charArray[pos] = e.key
        input.value = charArray.join('')
        
        let nextPos = pos + 1
        if (nextPos === 2 || nextPos === 5 || nextPos === 8) nextPos++
        input.setSelectionRange(nextPos, nextPos)
        onValueChange(true)
      }
    } else if (e.key === 'Backspace') {
      let pos = start - 1
      if (pos < 0) return
      if (pos === 2 || pos === 5 || pos === 8) pos--

      const charArray = value.split('')
      charArray[pos] = '0'
      input.value = charArray.join('')
      input.setSelectionRange(pos, pos)
      onValueChange(true)
    }
  })

  input.addEventListener('paste', (e) => {
    e.preventDefault()
    const text = e.clipboardData?.getData('text') ?? ''
    const digits = text.replace(/\D/g, '')
    
    let currentVal = input.value.split('')
    let currentPos = input.selectionStart ?? 0
    
    for (const digit of digits) {
      if (currentPos === 2 || currentPos === 5 || currentPos === 8) currentPos++
      if (currentPos >= format.length) break
      currentVal[currentPos] = digit
      currentPos++
    }
    
    input.value = currentVal.join('')
    onValueChange(true)
  })
}

/**
 * WAV Encoder helper.
 */
function audioBufferToWav(buffer: AudioBuffer): Blob {
  const numOfChan = buffer.numberOfChannels
  const length = buffer.length * numOfChan * 2 + 44
  const outBuffer = new ArrayBuffer(length)
  const view = new DataView(outBuffer)
  const channels = []
  let offset = 0
  let pos = 0

  setUint32(0x46464952)
  setUint32(length - 8)
  setUint32(0x45564157)
  setUint32(0x20746d66)
  setUint32(16)
  setUint16(1)
  setUint16(numOfChan)
  setUint32(buffer.sampleRate)
  setUint32(buffer.sampleRate * 2 * numOfChan)
  setUint16(numOfChan * 2)
  setUint16(16)
  setUint32(0x61746164)
  setUint32(length - pos - 4)

  for (let i = 0; i < buffer.numberOfChannels; i++) {
    channels.push(buffer.getChannelData(i))
  }

  while (pos < buffer.length) {
    for (let i = 0; i < numOfChan; i++) {
      let sample = Math.max(-1, Math.min(1, channels[i][pos]))
      sample = (sample < 0 ? sample * 0x8000 : sample * 0x7FFF)
      view.setInt16(offset, sample, true)
      offset += 2
    }
    pos++
  }

  return new Blob([outBuffer], { type: 'audio/wav' })

  function setUint16(data: number) {
    view.setUint16(offset, data, true)
    offset += 2
  }
  function setUint32(data: number) {
    view.setUint32(offset, data, true)
    offset += 4
  }
}

/**
 * Handles audio upload, waveform visualization, and trimming logic.
 */
function setupAudioApp() {
  const uploadBtn = document.getElementById('upload-btn')
  const audioInput = document.getElementById('audio-input') as HTMLInputElement
  const welcomeMessage = document.getElementById('welcome-message')
  const loadingContainer = document.getElementById('loading-container')
  const loadingFilenameSpan = document.getElementById('loading-filename')
  const loadingFileSizeSpan = document.getElementById('loading-file-size')
  const loadingEstimateSpan = document.getElementById('loading-estimate')
  const loadingTimeSpan = document.getElementById('loading-time')
  const loadingProgressBar = document.getElementById('loading-progress')
  const waveformContainer = document.getElementById('waveform-container')
  
  const trackLengthDisplay = document.getElementById('track-length')
  const currentTimestampInput = document.getElementById('current-timestamp') as HTMLInputElement
  
  const startTimeInput = document.getElementById('start-time') as HTMLInputElement
  const endTimeInput = document.getElementById('end-time') as HTMLInputElement
  const clipNameInput = document.getElementById('clip-name') as HTMLInputElement
  
  const playStartBtn = document.getElementById('play-start-btn')
  const playEndBtn = document.getElementById('play-end-btn')
  
  const startMinus1sBtn = document.getElementById('start-minus-1s-btn') as HTMLButtonElement
  const startMinus100msBtn = document.getElementById('start-minus-100ms-btn') as HTMLButtonElement
  const startPlus100msBtn = document.getElementById('start-plus-100ms-btn') as HTMLButtonElement
  const startPlus1sBtn = document.getElementById('start-plus-1s-btn') as HTMLButtonElement

  const endMinus1sBtn = document.getElementById('end-minus-1s-btn') as HTMLButtonElement
  const endMinus100msBtn = document.getElementById('end-minus-100ms-btn') as HTMLButtonElement
  const endPlus100msBtn = document.getElementById('end-plus-100ms-btn') as HTMLButtonElement
  const endPlus1sBtn = document.getElementById('end-plus-1s-btn') as HTMLButtonElement

  const setStartBtn = document.getElementById('set-start-btn') as HTMLButtonElement
  const setEndBtn = document.getElementById('set-end-btn') as HTMLButtonElement

  const clipBtn = document.getElementById('clip-btn')
  
  const errorDialog = document.getElementById('error-dialog')
  const errorMessage = document.getElementById('error-message')
  const errorCloseBtn = document.getElementById('error-close-btn')

  let wavesurfer: WaveSurfer | null = null
  let regions: any = null
  let currentFile: File | null = null
  let stopTime = 0
  let loadingInterval: number | null = null
  let loadingSeconds = 0
  let activePreview: 'start' | 'end' | null = null

  const getThemeColors = (theme: 'dark' | 'light') => {
    return theme === 'dark' 
      ? { wave: '#334155', progress: '#334155' }
      : { wave: '#e2e8f0', progress: '#e2e8f0' }
  }

  const currentTheme = () => document.documentElement.classList.contains('dark') ? 'dark' : 'light'

  const showError = (msg: string) => {
    if (errorMessage && errorDialog) {
      errorMessage.textContent = msg
      errorDialog.style.display = 'flex'
    }
  }

  errorCloseBtn?.addEventListener('click', () => {
    if (errorDialog) errorDialog.style.display = 'none'
  })

  const validateInputs = (isManual: boolean) => {
    if (!isManual || !wavesurfer) return
    
    const duration = wavesurfer.getDuration()
    const currentPos = parseTime(currentTimestampInput.value) || 0
    const startTime = parseTime(startTimeInput.value) || 0
    const endTime = parseTime(endTimeInput.value) || 0

    // Reset visual cues
    currentTimestampInput.classList.remove('invalid-input')
    endTimeInput.classList.remove('invalid-input')
    startTimeInput.classList.remove('invalid-input')
    trackLengthDisplay?.classList.remove('invalid-text')

    // Check Current Position vs Track Length
    if (currentPos > duration) {
      currentTimestampInput.classList.add('invalid-input')
      trackLengthDisplay?.classList.add('invalid-text')
    }

    // Check End Time vs Track Length
    if (endTime > duration) {
      endTimeInput.classList.add('invalid-input')
      trackLengthDisplay?.classList.add('invalid-text')
    }

    // Check Start Time vs End Time
    if (startTime > endTime) {
      startTimeInput.classList.add('invalid-input')
      endTimeInput.classList.add('invalid-input')
    }
  }

  const updatePlayIcons = (isPlaying: boolean, mode: 'start' | 'end' | null = null) => {
    const startIcon = playStartBtn?.querySelector('i')
    const endIcon = playEndBtn?.querySelector('i')

    if (startIcon) {
      startIcon.classList.remove('fa-pause')
      startIcon.classList.add('fa-play')
    }
    if (endIcon) {
      endIcon.classList.remove('fa-pause')
      endIcon.classList.add('fa-play')
    }

    if (isPlaying && mode) {
      const targetIcon = mode === 'start' ? startIcon : endIcon
      if (targetIcon) {
        targetIcon.classList.remove('fa-play')
        targetIcon.classList.add('fa-pause')
      }
    }
  }

  const startLoadingTimer = () => {
    loadingSeconds = 0
    if (loadingTimeSpan) loadingTimeSpan.textContent = `(0 s)`
    if (loadingInterval) clearInterval(loadingInterval)
    loadingInterval = window.setInterval(() => {
      loadingSeconds++
      if (loadingTimeSpan) loadingTimeSpan.textContent = `(${loadingSeconds} s)`
    }, 1000)
  }

  const stopLoadingTimer = () => {
    if (loadingInterval) {
      clearInterval(loadingInterval)
      loadingInterval = null
    }
  }

  const updateButtonStates = () => {
    if (!wavesurfer) return
    const duration = wavesurfer.getDuration()

    const startTime = parseTime(startTimeInput.value) || 0
    const endTime = parseTime(endTimeInput.value) || duration

    if (startMinus1sBtn) startMinus1sBtn.disabled = startTime < 1
    if (startMinus100msBtn) startMinus100msBtn.disabled = startTime < 0.1
    if (startPlus100msBtn) startPlus100msBtn.disabled = startTime > duration - 0.1
    if (startPlus1sBtn) startPlus1sBtn.disabled = startTime > duration - 1

    if (endMinus1sBtn) endMinus1sBtn.disabled = endTime < 1
    if (endMinus100msBtn) endMinus100msBtn.disabled = endTime < 0.1
    if (endPlus100msBtn) endPlus100msBtn.disabled = endTime > duration - 0.1
    if (endPlus1sBtn) endPlus1sBtn.disabled = endTime > duration - 1
  }

  const updateRegion = () => {
    if (!regions) return
    const start = parseTime(startTimeInput.value) || 0
    const end = parseTime(endTimeInput.value) || (wavesurfer?.getDuration() || 0)
    
    const region = regions.getRegions().find((r: any) => r.id === 'selection')
    if (region) {
      region.setOptions({ start, end })
    }
  }

  const handleTimeChange = (input: HTMLInputElement, delta: number) => {
    const currentTime = parseTime(input.value) || 0
    const newTime = Math.max(0, Math.min(wavesurfer?.getDuration() || 0, currentTime + delta))
    input.value = formatTimeWithMs(newTime)
    
    // Clear error states on programmatic change
    input.classList.remove('invalid-input')
    if (input === startTimeInput || input === endTimeInput) {
      startTimeInput.classList.remove('invalid-input')
      endTimeInput.classList.remove('invalid-input')
    }
    if (input === currentTimestampInput || input === endTimeInput) {
      trackLengthDisplay?.classList.remove('invalid-text')
    }

    updateButtonStates()
    updateRegion()
  }

  const initWaveSurfer = () => {
    const colors = getThemeColors(currentTheme())
    
    wavesurfer = WaveSurfer.create({
      container: '#waveform',
      waveColor: colors.wave,
      progressColor: colors.progress,
      cursorColor: currentTheme() === 'dark' ? '#475569' : '#94a3b8',
      height: 128,
    })

    regions = wavesurfer.registerPlugin(RegionsPlugin.create())

    wavesurfer.on('ready', () => {
      stopLoadingTimer()
      
      if (loadingProgressBar) {
        loadingProgressBar.style.transition = 'width 0.2s ease-out'
        loadingProgressBar.style.width = '100%'
      }

      setTimeout(() => {
        if (loadingContainer) loadingContainer.style.display = 'none'
        if (waveformContainer) waveformContainer.style.display = 'block'
      }, 300)
      
      const duration = wavesurfer?.getDuration() || 0
      if (trackLengthDisplay) {
        trackLengthDisplay.textContent = formatTimeWithMs(duration)
      }
      if (endTimeInput) {
        endTimeInput.value = formatTimeWithMs(duration)
      }

      regions.clearRegions()
      regions.addRegion({
        id: 'selection',
        start: 0,
        end: duration,
        color: 'rgba(59, 130, 246, 0.4)',
        drag: false,
        resize: false,
      })

      updateButtonStates()
    })

    wavesurfer.on('play', () => updatePlayIcons(true, activePreview))
    wavesurfer.on('pause', () => {
      activePreview = null
      updatePlayIcons(false)
    })
    wavesurfer.on('finish', () => {
      activePreview = null
      updatePlayIcons(false)
    })

    wavesurfer.on('timeupdate', (currentTime) => {
      if (currentTimestampInput && document.activeElement !== currentTimestampInput) {
        currentTimestampInput.value = formatTimeWithMs(currentTime)
      }
      if (wavesurfer?.isPlaying() && currentTime >= stopTime) {
        wavesurfer.pause()
      }
    })

    wavesurfer.on('interaction', (newTime) => {
       if (currentTimestampInput) {
         currentTimestampInput.value = formatTimeWithMs(newTime)
         // interaction clears red
         currentTimestampInput.classList.remove('invalid-input')
         trackLengthDisplay?.classList.remove('invalid-text')
       }
    })
  }

  const togglePlayback = (mode: 'start' | 'end') => {
    if (!wavesurfer) return

    const start = parseTime(startTimeInput.value)
    const end = parseTime(endTimeInput.value)

    if (start === null || end === null || startTimeInput.value.trim() === '' || endTimeInput.value.trim() === '') {
      showError('Please enter valid Start and End times in HH:MM:SS:mmm format.')
      return
    }

    if (start >= end) {
      showError('Start time must be less than end time.')
      return
    }

    if (wavesurfer.isPlaying()) {
      if (activePreview === mode) {
        wavesurfer.pause()
        activePreview = null
        updatePlayIcons(false)
        return
      } else {
        wavesurfer.pause()
      }
    }

    activePreview = mode
    stopTime = end
    
    if (mode === 'end') {
      const previewStart = Math.max(start, end - 2)
      wavesurfer.setTime(previewStart)
    } else {
      wavesurfer.setTime(start)
    }
    
    wavesurfer.play()
    updatePlayIcons(true, mode)
  }

  // Setup input masks
  setupTimeInputMask(currentTimestampInput, (isManual) => {
    const time = parseTime(currentTimestampInput.value)
    if (time !== null && wavesurfer && isManual) {
      if (time <= wavesurfer.getDuration()) {
        wavesurfer.setTime(time)
      }
    }
    validateInputs(isManual)
  })
  setupTimeInputMask(startTimeInput, (isManual) => {
    validateInputs(isManual)
    if (isManual) {
      const start = parseTime(startTimeInput.value)
      const end = parseTime(endTimeInput.value)
      if (start !== null && end !== null && start <= end) {
        updateButtonStates()
        updateRegion()
      }
    }
  })
  setupTimeInputMask(endTimeInput, (isManual) => {
    validateInputs(isManual)
    if (isManual) {
      const start = parseTime(startTimeInput.value)
      const end = parseTime(endTimeInput.value)
      const duration = wavesurfer?.getDuration() || 0
      if (start !== null && end !== null && start <= end && end <= duration) {
        updateButtonStates()
        updateRegion()
      }
    }
  })

  uploadBtn?.addEventListener('click', () => {
    audioInput?.click()
  })

  audioInput?.addEventListener('change', (event) => {
    const file = (event.target as HTMLInputElement).files?.[0]
    if (file) {
      currentFile = file
      if (!wavesurfer) {
        initWaveSurfer()
      }

      if (welcomeMessage) welcomeMessage.style.display = 'none'
      if (waveformContainer) waveformContainer.style.display = 'none'
      if (loadingContainer) {
        if (startTimeInput) startTimeInput.value = '00:00:00:000'
        if (currentTimestampInput) currentTimestampInput.value = '00:00:00:000'
        if (wavesurfer) wavesurfer.setTime(0)
        
        // Reset validation cues on new file
        startTimeInput.classList.remove('invalid-input')
        endTimeInput.classList.remove('invalid-input')
        currentTimestampInput.classList.remove('invalid-input')
        trackLengthDisplay?.classList.remove('invalid-text')

        if (loadingProgressBar) {
          loadingProgressBar.style.transition = 'none'
          loadingProgressBar.style.width = '0%'
        }

        if (loadingFilenameSpan) {
          loadingFilenameSpan.textContent = file.name
        }

        const sizeMB = file.size / (1024 * 1024)
        if (loadingFileSizeSpan) {
          loadingFileSizeSpan.textContent = formatFileSize(file.size)
        }

        const ext = file.name.split('.').pop()?.toLowerCase() || ''
        const factor = (ext === 'wav' || ext === 'aiff') ? 0.12 : 0.72
        const totalEstimate = Math.max(1, Math.ceil(sizeMB * factor))
        
        if (loadingEstimateSpan) {
          loadingEstimateSpan.textContent = `${totalEstimate}`
        }

        loadingContainer.style.display = 'block'
        
        setTimeout(() => {
          if (loadingProgressBar) {
            loadingProgressBar.style.transition = `width ${totalEstimate}s linear`
            loadingProgressBar.style.width = '98%'
          }
        }, 50)

        startLoadingTimer()
      }

      const url = URL.createObjectURL(file)
      wavesurfer?.load(url)
    }
  })

  playStartBtn?.addEventListener('click', () => togglePlayback('start'))
  playEndBtn?.addEventListener('click', () => togglePlayback('end'))

  startMinus1sBtn?.addEventListener('click', () => handleTimeChange(startTimeInput, -1))
  startMinus100msBtn?.addEventListener('click', () => handleTimeChange(startTimeInput, -0.1))
  startPlus100msBtn?.addEventListener('click', () => handleTimeChange(startTimeInput, 0.1))
  startPlus1sBtn?.addEventListener('click', () => handleTimeChange(startTimeInput, 1))

  endMinus1sBtn?.addEventListener('click', () => handleTimeChange(endTimeInput, -1))
  endMinus100msBtn?.addEventListener('click', () => handleTimeChange(endTimeInput, -0.1))
  endPlus100msBtn?.addEventListener('click', () => handleTimeChange(endTimeInput, 0.1))
  endPlus1sBtn?.addEventListener('click', () => handleTimeChange(endTimeInput, 1))

  setStartBtn?.addEventListener('click', () => {
    if (!wavesurfer) return
    const currentTime = wavesurfer.getCurrentTime()
    startTimeInput.value = formatTimeWithMs(currentTime)

    // Programmatic change clears red
    startTimeInput.classList.remove('invalid-input')
    endTimeInput.classList.remove('invalid-input')

    const endTime = parseTime(endTimeInput.value) || 0
    if (endTime < currentTime) {
      const duration = wavesurfer.getDuration()
      const newEndTime = Math.min(currentTime + 60, duration)
      endTimeInput.value = formatTimeWithMs(newEndTime)
    }
    
    updateButtonStates()
    updateRegion()
  })

  setEndBtn?.addEventListener('click', () => {
    if (!wavesurfer) return
    const currentTime = wavesurfer.getCurrentTime()
    endTimeInput.value = formatTimeWithMs(currentTime)

    // Programmatic change clears red
    startTimeInput.classList.remove('invalid-input')
    endTimeInput.classList.remove('invalid-input')
    trackLengthDisplay?.classList.remove('invalid-text')

    const startTime = parseTime(startTimeInput.value) || 0
    if (startTime > currentTime) {
      const newStartTime = Math.max(currentTime - 60, 0)
      startTimeInput.value = formatTimeWithMs(newStartTime)
    }
    
    updateButtonStates()
    updateRegion()
  })

  clipBtn?.addEventListener('click', async () => {
    if (!currentFile || !wavesurfer) return

    const start = parseTime(startTimeInput.value)
    const end = parseTime(endTimeInput.value)
    const name = clipNameInput.value.trim()

    if (start === null || end === null || name === '' || startTimeInput.value.trim() === '' || endTimeInput.value.trim() === '') {
      showError('Start Time, End Time, and Clip Name cannot be blank and must be valid.')
      return
    }

    if (start >= end) {
      showError('Start time must be less than end time.')
      return
    }

    try {
      clipBtn.innerHTML = '<i class="fas fa-spinner fa-spin"></i> <span>Clipping...</span>'
      ;(clipBtn as HTMLButtonElement).disabled = true

      const audioCtx = new (window.AudioContext || (window as any).webkitAudioContext)()
      const arrayBuffer = await currentFile.arrayBuffer()
      const audioBuffer = await audioCtx.decodeAudioData(arrayBuffer)

      const sampleRate = audioBuffer.sampleRate
      const startOffset = Math.floor(start * sampleRate)
      const endOffset = Math.floor(end * sampleRate)
      const frameCount = endOffset - startOffset

      const newBuffer = audioCtx.createBuffer(
        audioBuffer.numberOfChannels,
        frameCount,
        sampleRate
      )

      for (let i = 0; i < audioBuffer.numberOfChannels; i++) {
        const channelData = audioBuffer.getChannelData(i)
        const newChannelData = newBuffer.getChannelData(i)
        newChannelData.set(channelData.subarray(startOffset, endOffset))
      }

      const wavBlob = audioBufferToWav(newBuffer)
      const url = URL.createObjectURL(wavBlob)
      
      const link = document.createElement('a')
      link.href = url
      link.download = `${name}.wav`
      document.body.appendChild(link)
      link.click()
      document.body.removeChild(link)
      URL.revokeObjectURL(url)

    } catch (err) {
      console.error(err)
      showError('Failed to create audio clip.')
    } finally {
      clipBtn.innerHTML = '<i class="fas fa-cut"></i> <span>Clip</span>'
      ;(clipBtn as HTMLButtonElement).disabled = false
    }
  })

  setupThemeToggle((theme) => {
    if (wavesurfer) {
      const colors = getThemeColors(theme)
      wavesurfer.setOptions({
        waveColor: colors.wave,
        progressColor: colors.progress,
        cursorColor: theme === 'dark' ? '#475569' : '#94a3b8',
      })
    }
  })
}

if (document.readyState === 'loading') {
  document.addEventListener('DOMContentLoaded', setupAudioApp)
} else {
  setupAudioApp()
}
