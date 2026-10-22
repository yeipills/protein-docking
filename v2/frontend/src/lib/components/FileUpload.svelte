<!-- File Upload Component - Upload de proteínas con drag & drop -->
<script lang="ts">
  import { api } from '../stores/api.svelte';
  import { notificationService } from '../services/NotificationService';
  import type { UploadResponse } from '../services/ApiService';

  interface Props {
    accept?: string;
    maxSize?: number;
    onUpload?: (response: UploadResponse) => void;
  }

  let {
    accept = '.pdb,.mol,.sdf,.mol2',
    maxSize = 500 * 1024 * 1024, // 500MB
    onUpload
  }: Props = $props();

  let dragover = $state(false);
  let uploading = $state<string[]>([]);
  let errors = $state<string[]>([]);

  // File input reference
  let fileInput: HTMLInputElement;

  // Supported formats info
  const supportedFormats = [
    { ext: '.pdb', desc: 'Protein Data Bank' },
    { ext: '.mol', desc: 'MDL Molfile' },
    { ext: '.sdf', desc: 'Structure Data File' },
    { ext: '.mol2', desc: 'Tripos MOL2' }
  ];

  function handleDragOver(event: DragEvent) {
    event.preventDefault();
    dragover = true;
  }

  function handleDragLeave(event: DragEvent) {
    event.preventDefault();
    dragover = false;
  }

  async function handleDrop(event: DragEvent) {
    event.preventDefault();
    dragover = false;

    const files = Array.from(event.dataTransfer?.files || []);
    await uploadFiles(files);
  }

  function handleFileSelect(event: Event) {
    const target = event.target as HTMLInputElement;
    const files = Array.from(target.files || []);
    uploadFiles(files);
    
    // Reset input
    target.value = '';
  }

  function validateFile(file: File): string | null {
    // Check file size
    if (file.size > maxSize) {
      return `File too large: ${formatFileSize(file.size)} (max: ${formatFileSize(maxSize)})`;
    }

    // Check file extension
    const extension = file.name.toLowerCase().split('.').pop();
    const allowedExtensions = accept.split(',').map(ext => ext.trim().replace('.', ''));
    
    if (!extension || !allowedExtensions.includes(extension)) {
      return `Unsupported format: .${extension}. Supported: ${allowedExtensions.map(ext => `.${ext}`).join(', ')}`;
    }

    return null;
  }

  async function uploadFiles(files: File[]) {
    errors = [];
    
    for (const file of files) {
      const validationError = validateFile(file);
      if (validationError) {
        errors.push(`${file.name}: ${validationError}`);
        notificationService.fileUploadFailed(file.name, validationError);
        continue;
      }

      let notificationId: string;
      try {
        uploading.push(file.name);
        uploading = [...uploading]; // Trigger reactivity

        // Mostrar notificación de inicio
        notificationId = notificationService.showProgress(
          'Uploading file',
          `Uploading ${file.name}...`,
          0
        );

        const response = await api.uploadFile(file, (percent) => {
          // Actualizar progreso en notificación
          notificationService.updateProgress(notificationId, percent, `Uploading ${file.name}... ${percent.toFixed(1)}%`);
          console.log(`Upload progress: ${file.name} - ${percent}%`);
        });

        if (response) {
          console.log(`FileUpload: About to call onUpload with response:`, response);
          onUpload?.(response);
          notificationService.fileUploaded(file.name, response.file_id);
          console.log(`✅ Successfully uploaded: ${file.name}`);
        }

      } catch (error) {
        console.error(`❌ Upload failed: ${file.name}`, error);
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        errors.push(`${file.name}: Upload failed - ${errorMessage}`);
        notificationService.fileUploadFailed(file.name, errorMessage);
      } finally {
        uploading = uploading.filter(name => name !== file.name);
      }
    }
  }

  function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  }

  function triggerFileSelect() {
    fileInput?.click();
  }

  // Progress is automatically displayed via the store reactive state
</script>

<!-- File Upload Component -->
<div class="file-upload">
  <!-- Hidden file input -->
  <input
    bind:this={fileInput}
    type="file"
    {accept}
    multiple
    style="display: none"
    onchange={handleFileSelect}
  />

  <!-- Drop zone -->
  <div
    class="drop-zone"
    class:dragover
    class:uploading={uploading.length > 0}
    ondragover={handleDragOver}
    ondragleave={handleDragLeave}
    ondrop={handleDrop}
    onclick={triggerFileSelect}
    role="button"
    tabindex="0"
    onkeydown={(e) => {
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        triggerFileSelect();
      }
    }}
  >
    <div class="drop-content">
      {#if uploading.length > 0}
        <div class="upload-status">
          <div class="spinner"></div>
          <h3>Uploading {uploading.length} file{uploading.length > 1 ? 's' : ''}...</h3>
          
          {#each uploading as filename}
            {@const progress = api.uploadProgress.get(filename) || 0}
            <div class="upload-item">
              <span class="filename">{filename}</span>
              <div class="progress-bar">
                <div class="progress-fill" style="width: {progress}%"></div>
              </div>
              <span class="progress-text">{progress}%</span>
            </div>
          {/each}
        </div>
      {:else}
        <div class="upload-prompt">
          <svg class="upload-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
            <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
            <polyline points="7,10 12,15 17,10"></polyline>
            <line x1="12" y1="15" x2="12" y2="3"></line>
          </svg>
          
          <h3>Drop protein files here</h3>
          <p>or <span class="link">click to browse</span></p>
          
          <div class="file-info">
            <p>Maximum file size: <strong>{formatFileSize(maxSize)}</strong></p>
            <p>Supported formats:</p>
            <ul class="format-list">
              {#each supportedFormats as format}
                <li><code>{format.ext}</code> - {format.desc}</li>
              {/each}
            </ul>
          </div>
        </div>
      {/if}
    </div>
  </div>

  <!-- Connection status -->
  {#if !api.state.connected}
    <div class="connection-warning">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"></path>
        <line x1="12" y1="9" x2="12" y2="13"></line>
        <line x1="12" y1="17" x2="12.01" y2="17"></line>
      </svg>
      Backend not connected. Upload will fail.
    </div>
  {/if}

  <!-- Error messages -->
  {#if errors.length > 0}
    <div class="error-list">
      <h4>Upload Errors:</h4>
      {#each errors as error}
        <div class="error-item">{error}</div>
      {/each}
      <button onclick={() => errors = []} class="clear-errors">Clear Errors</button>
    </div>
  {/if}
</div>

<style>
  .file-upload {
    width: 100%;
    max-width: 600px;
    margin: 0 auto;
  }

  .drop-zone {
    border: 2px dashed #d1d5db;
    border-radius: 12px;
    padding: 2rem;
    text-align: center;
    cursor: pointer;
    transition: all 0.3s ease;
    background: #f9fafb;
    min-height: 200px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .drop-zone:hover,
  .drop-zone:focus {
    border-color: #3b82f6;
    background: #eff6ff;
    outline: none;
  }

  .drop-zone.dragover {
    border-color: #10b981;
    background: #ecfdf5;
    transform: scale(1.02);
  }

  .drop-zone.uploading {
    border-color: #f59e0b;
    background: #fffbeb;
  }

  .drop-content {
    width: 100%;
  }

  .upload-icon {
    width: 48px;
    height: 48px;
    margin: 0 auto 1rem;
    color: #6b7280;
  }

  .upload-prompt h3 {
    margin: 0 0 0.5rem;
    color: #374151;
    font-size: 1.25rem;
    font-weight: 600;
  }

  .upload-prompt p {
    margin: 0 0 1.5rem;
    color: #6b7280;
  }

  .link {
    color: #3b82f6;
    text-decoration: underline;
    font-weight: 500;
  }

  .file-info {
    padding-top: 1rem;
    border-top: 1px solid #e5e7eb;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .file-info p {
    margin: 0.5rem 0;
  }

  .format-list {
    list-style: none;
    padding: 0;
    margin: 0.5rem 0 0;
    display: grid;
    grid-template-columns: repeat(2, 1fr);
    gap: 0.25rem;
  }

  .format-list code {
    background: #f3f4f6;
    padding: 0.125rem 0.25rem;
    border-radius: 0.25rem;
    font-weight: 600;
    color: #1f2937;
  }

  /* Upload status */
  .upload-status h3 {
    margin: 0 0 1rem;
    color: #f59e0b;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    justify-content: center;
  }

  .spinner {
    width: 20px;
    height: 20px;
    border: 2px solid #fbbf24;
    border-top-color: transparent;
    border-radius: 50%;
    animation: spin 1s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .upload-item {
    display: grid;
    grid-template-columns: 1fr auto auto;
    gap: 1rem;
    align-items: center;
    padding: 0.5rem 0;
    border-bottom: 1px solid #f3f4f6;
  }

  .upload-item:last-child {
    border-bottom: none;
  }

  .filename {
    font-weight: 500;
    color: #374151;
    text-align: left;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .progress-bar {
    width: 120px;
    height: 8px;
    background: #e5e7eb;
    border-radius: 4px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: #10b981;
    transition: width 0.3s ease;
    border-radius: 4px;
  }

  .progress-text {
    font-size: 0.875rem;
    font-weight: 500;
    color: #6b7280;
    min-width: 3rem;
    text-align: right;
  }

  /* Connection warning */
  .connection-warning {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-top: 1rem;
    padding: 0.75rem;
    background: #fef3cd;
    border: 1px solid #f59e0b;
    border-radius: 8px;
    color: #92400e;
    font-size: 0.875rem;
  }

  .connection-warning svg {
    width: 16px;
    height: 16px;
    flex-shrink: 0;
  }

  /* Error messages */
  .error-list {
    margin-top: 1rem;
    padding: 1rem;
    background: #fef2f2;
    border: 1px solid #f87171;
    border-radius: 8px;
  }

  .error-list h4 {
    margin: 0 0 0.5rem;
    color: #dc2626;
    font-size: 0.875rem;
    font-weight: 600;
  }

  .error-item {
    padding: 0.25rem 0;
    color: #b91c1c;
    font-size: 0.875rem;
    border-bottom: 1px solid #fecaca;
  }

  .error-item:last-of-type {
    border-bottom: none;
  }

  .clear-errors {
    margin-top: 0.75rem;
    padding: 0.5rem 1rem;
    background: #dc2626;
    color: white;
    border: none;
    border-radius: 6px;
    font-size: 0.875rem;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .clear-errors:hover {
    background: #b91c1c;
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    .drop-zone {
      background: #1f2937;
      border-color: #4b5563;
      color: #e5e7eb;
    }

    .drop-zone:hover,
    .drop-zone:focus {
      background: #374151;
      border-color: #60a5fa;
    }

    .drop-zone.dragover {
      background: #064e3b;
      border-color: #34d399;
    }

    .upload-icon {
      color: #9ca3af;
    }

    .upload-prompt h3 {
      color: #e5e7eb;
    }

    .upload-prompt p {
      color: #9ca3af;
    }

    .file-info {
      border-top-color: #4b5563;
      color: #9ca3af;
    }

    .format-list code {
      background: #374151;
      color: #e5e7eb;
    }
  }
</style>