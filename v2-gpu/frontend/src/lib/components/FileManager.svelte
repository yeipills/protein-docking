<!-- File Manager Component - Gestión de archivos de proteínas subidos -->
<script lang="ts">
  import { api } from '../stores/api.svelte';
  import type { FileMetadata } from '../services/ApiService';

  interface Props {
    onSelect?: (file: FileMetadata) => void;
    selectionMode?: 'single' | 'multiple';
    filterType?: 'receptor' | 'ligand' | 'all';
  }

  let {
    onSelect,
    selectionMode = 'single',
    filterType = 'all'
  }: Props = $props();

  let selectedFiles = $state<Set<string>>(new Set());
  let sortBy = $state<'name' | 'size' | 'date'>('date');
  let sortOrder = $state<'asc' | 'desc'>('desc');
  let searchQuery = $state('');

  // REEMPLAZAR $derived problemático con $state manual
  let filteredFiles = $state<FileMetadata[]>([]);
  
  // Function to recompute filtered files
  function recomputeFilteredFiles() {
    const fileCount = api.state.files.length;
    const allFiles = api.state.files;
    const currentQuery = searchQuery;
    const currentFilter = filterType;
    
    console.log('FileManager: MANUAL recomputing with', fileCount, 'files');
    console.log('FileManager: Raw files data:', allFiles);
    console.log('FileManager: Current search query:', currentQuery);
    console.log('FileManager: Current filter type:', currentFilter);
    
    if (fileCount === 0) {
      console.log('FileManager: No files to filter, setting empty array');
      filteredFiles = [];
      return;
    }
    
    let files = [...allFiles]; // Clone para evitar mutaciones

    // Filter by type
    if (filterType === 'receptor') {
      files = files.filter(f => f.protein_info && f.protein_info.total_atoms > 100);
    } else if (filterType === 'ligand') {
      files = files.filter(f => f.protein_info && f.protein_info.total_atoms <= 100);
    }

    // Filter by search query
    if (searchQuery) {
      const query = searchQuery.toLowerCase();
      files = files.filter(file => 
        file.filename.toLowerCase().includes(query) ||
        file.protein_info?.name?.toLowerCase().includes(query) ||
        file.protein_info?.pdb_id?.toLowerCase().includes(query)
      );
    }

    // Sort files
    files.sort((a, b) => {
      let comparison = 0;
      
      switch (sortBy) {
        case 'name':
          comparison = a.filename.localeCompare(b.filename);
          break;
        case 'size':
          comparison = a.file_size - b.file_size;
          break;
        case 'date':
          comparison = new Date(a.upload_timestamp).getTime() - new Date(b.upload_timestamp).getTime();
          break;
      }
      
      return sortOrder === 'asc' ? comparison : -comparison;
    });

    console.log('FileManager: MANUAL filtered to', files.length, 'files');
    filteredFiles = files;
  }

  function handleFileSelect(file: FileMetadata) {
    if (selectionMode === 'single') {
      selectedFiles.clear();
      selectedFiles.add(file.file_id);
      onSelect?.(file);
    } else {
      if (selectedFiles.has(file.file_id)) {
        selectedFiles.delete(file.file_id);
      } else {
        selectedFiles.add(file.file_id);
      }
      selectedFiles = new Set(selectedFiles); // Trigger reactivity
      onSelect?.(file);
    }
  }

  function handleSort(newSortBy: 'name' | 'size' | 'date') {
    if (sortBy === newSortBy) {
      sortOrder = sortOrder === 'asc' ? 'desc' : 'asc';
    } else {
      sortBy = newSortBy;
      sortOrder = 'desc';
    }
    recomputeFilteredFiles();
  }

  async function handleDelete(file: FileMetadata, event: Event) {
    event.stopPropagation();
    
    if (!confirm(`Are you sure you want to delete "${file.filename}"?`)) {
      return;
    }

    const success = await api.deleteFile(file.file_id);
    if (success) {
      selectedFiles.delete(file.file_id);
      selectedFiles = new Set(selectedFiles);
    }
  }

  async function handleDownload(file: FileMetadata, event: Event) {
    event.stopPropagation();
    
    try {
      await api.downloadFile(file.file_id, file.filename);
    } catch (error) {
      alert(`Download failed: ${error instanceof Error ? error.message : 'Unknown error'}`);
    }
  }

  function formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  }

  function formatDate(dateString: string): string {
    const date = new Date(dateString);
    return date.toLocaleString();
  }

  function getFileTypeIcon(filename: string): string {
    const ext = filename.toLowerCase().split('.').pop();
    switch (ext) {
      case 'pdb': return '🧬';
      case 'mol': return '⚛️';
      case 'sdf': return '📊';
      case 'mol2': return '🔬';
      default: return '📄';
    }
  }

  function getProteinTypeLabel(file: FileMetadata): string {
    if (!file.protein_info) return 'Unknown';
    
    const atoms = file.protein_info.total_atoms;
    if (atoms > 1000) return 'Large Protein';
    if (atoms > 100) return 'Protein';
    if (atoms > 50) return 'Peptide';
    return 'Small Molecule';
  }

  function clearSelection() {
    selectedFiles.clear();
    selectedFiles = new Set();
  }

  async function refreshFiles() {
    console.log('FileManager: Manual refresh triggered');
    console.log('FileManager: Current state before refresh:', api.state.files);
    
    // Force trigger state update
    await api.loadFiles();
    
    console.log('FileManager: Manual refresh completed, files:', api.state.files);
    recomputeFilteredFiles();
    console.log('FileManager: Filtered files length after refresh:', filteredFiles.length);
  }
  
  // Debug: Log cuando el componente se monta
  import { onMount } from 'svelte';
  
  // Force reactivity with effect
  let forceUpdate = $state(0);
  
  $effect(() => {
    // Re-compute filtered files when files change
    const currentFileCount = api.state.files.length;
    console.log('FileManager: Effect triggered, file count:', currentFileCount);
    recomputeFilteredFiles();
  });
  
  // Also recompute when search/filter changes
  $effect(() => {
    console.log('FileManager: Search/filter changed, recomputing...');
    recomputeFilteredFiles();
  });
  
  onMount(() => {
    console.log('FileManager: Component mounted, initial files:', api.state.files);
    console.log('FileManager: Initial filteredFiles.length:', filteredFiles.length);
    
    // Initial computation
    recomputeFilteredFiles();
    console.log('FileManager: After initial computation:', filteredFiles.length);
    
    // Add periodic check to see if files change
    const interval = setInterval(() => {
      console.log('FileManager: Periodic check - files count:', api.state.files.length);
      console.log('FileManager: Periodic check - filteredFiles.length:', filteredFiles.length);
    }, 2000);
    
    return () => clearInterval(interval);
  });
</script>

<!-- File Manager Component -->
<div class="file-manager">
  <!-- Header -->
  <div class="manager-header">
    <div class="header-left">
      <h3>Uploaded Files ({filteredFiles.length})</h3>
      {#if selectedFiles.size > 0}
        <span class="selection-info">
          {selectedFiles.size} selected
          <button onclick={clearSelection} class="clear-selection">Clear</button>
        </span>
      {/if}
    </div>
    
    <div class="header-right">
      <button onclick={refreshFiles} class="refresh-btn" title="Refresh files" aria-label="Refresh files list">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path d="M21.5 2v6h-6"></path>
          <path d="M2.5 22v-6h6"></path>
          <path d="M21.5 8a10 10 0 0 0-18.8-4.3"></path>
          <path d="M2.5 16a10 10 0 0 0 18.8 4.3"></path>
        </svg>
      </button>
    </div>
  </div>

  <!-- Search and filters -->
  <div class="controls">
    <div class="search-box">
      <svg class="search-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor">
        <circle cx="11" cy="11" r="8"></circle>
        <path d="M21 21l-4.35-4.35"></path>
      </svg>
      <input
        type="text"
        placeholder="Search files..."
        bind:value={searchQuery}
        class="search-input"
      />
    </div>

    <div class="filter-type">
      <label for="filter-select">Filter:</label>
      <select id="filter-select" bind:value={filterType}>
        <option value="all">All Files</option>
        <option value="receptor">Receptors (Large)</option>
        <option value="ligand">Ligands (Small)</option>
      </select>
    </div>
  </div>

  <!-- File list -->
  {#if api.state.connecting}
    <div class="loading-state">
      <div class="spinner"></div>
      <p>Loading files...</p>
    </div>
  {:else if filteredFiles.length === 0}
    <div class="empty-state">
      {#if api.state.files.length === 0}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"></path>
          <polyline points="14,2 14,8 20,8"></polyline>
          <line x1="16" y1="13" x2="8" y2="13"></line>
          <line x1="16" y1="17" x2="8" y2="17"></line>
          <polyline points="10,9 9,9 8,9"></polyline>
        </svg>
        <h4>No files uploaded</h4>
        <p>Upload your first protein file to get started</p>
      {:else}
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
          <circle cx="11" cy="11" r="8"></circle>
          <path d="M21 21l-4.35-4.35"></path>
        </svg>
        <h4>No files match your search</h4>
        <p>Try adjusting your search terms or filters</p>
      {/if}
    </div>
  {:else}
    <div class="file-list">
      <!-- Table header -->
      <div class="file-header">
        <button
          class="sort-btn"
          class:active={sortBy === 'name'}
          onclick={() => handleSort('name')}
        >
          File Name
          {#if sortBy === 'name'}
            <span class="sort-indicator">{sortOrder === 'asc' ? '↑' : '↓'}</span>
          {/if}
        </button>

        <span class="header-cell">Type</span>
        <span class="header-cell">Protein Info</span>

        <button
          class="sort-btn"
          class:active={sortBy === 'size'}
          onclick={() => handleSort('size')}
        >
          Size
          {#if sortBy === 'size'}
            <span class="sort-indicator">{sortOrder === 'asc' ? '↑' : '↓'}</span>
          {/if}
        </button>

        <button
          class="sort-btn"
          class:active={sortBy === 'date'}
          onclick={() => handleSort('date')}
        >
          Upload Date
          {#if sortBy === 'date'}
            <span class="sort-indicator">{sortOrder === 'asc' ? '↑' : '↓'}</span>
          {/if}
        </button>

        <span class="header-cell">Actions</span>
      </div>

      <!-- File rows -->
      {#each filteredFiles as file (file.file_id)}
        <div
          class="file-row"
          class:selected={selectedFiles.has(file.file_id)}
          onclick={() => handleFileSelect(file)}
          role="button"
          tabindex="0"
          onkeydown={(e) => {
            if (e.key === 'Enter' || e.key === ' ') {
              e.preventDefault();
              handleFileSelect(file);
            }
          }}
        >
          <div class="file-name">
            <span class="file-icon">{getFileTypeIcon(file.filename)}</span>
            <div class="name-info">
              <span class="filename">{file.filename}</span>
              <span class="file-id">{file.file_id}</span>
            </div>
          </div>

          <div class="file-type">
            <span class="type-label">{getProteinTypeLabel(file)}</span>
            <span class="format">.{file.filename.split('.').pop()?.toUpperCase()}</span>
          </div>

          <div class="protein-info">
            {#if file.protein_info}
              <div class="protein-stats">
                <span class="stat">
                  <strong>{file.protein_info.total_atoms}</strong> atoms
                </span>
                <span class="stat">
                  <strong>{file.protein_info.total_residues}</strong> residues
                </span>
                <span class="stat">
                  <strong>{file.protein_info.chains}</strong> chains
                </span>
                {#if file.protein_info.pdb_id}
                  <span class="pdb-id">PDB: {file.protein_info.pdb_id}</span>
                {/if}
              </div>
            {:else}
              <span class="no-info">Not parsed</span>
            {/if}
          </div>

          <div class="file-size">
            {formatFileSize(file.file_size)}
          </div>

          <div class="upload-date">
            {formatDate(file.upload_timestamp)}
          </div>

          <div class="file-actions">
            <button
              onclick={(e) => handleDownload(file, e)}
              class="action-btn download"
              title="Download file"
              aria-label="Download {file.filename}"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"></path>
                <polyline points="7,10 12,15 17,10"></polyline>
                <line x1="12" y1="15" x2="12" y2="3"></line>
              </svg>
            </button>

            <button
              onclick={(e) => handleDelete(file, e)}
              class="action-btn delete"
              title="Delete file"
              aria-label="Delete {file.filename}"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor">
                <polyline points="3,6 5,6 21,6"></polyline>
                <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"></path>
                <line x1="10" y1="11" x2="10" y2="17"></line>
                <line x1="14" y1="11" x2="14" y2="17"></line>
              </svg>
            </button>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .file-manager {
    width: 100%;
    background: white;
    border: 1px solid #e5e7eb;
    border-radius: 12px;
    overflow: hidden;
  }

  .manager-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .manager-header h3 {
    margin: 0;
    font-size: 1.125rem;
    font-weight: 600;
    color: #111827;
  }

  .selection-info {
    font-size: 0.875rem;
    color: #6b7280;
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .clear-selection {
    background: none;
    border: none;
    color: #3b82f6;
    cursor: pointer;
    text-decoration: underline;
    font-size: 0.875rem;
  }

  .refresh-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 36px;
    height: 36px;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .refresh-btn:hover {
    background: #f3f4f6;
    border-color: #9ca3af;
  }

  .refresh-btn svg {
    width: 16px;
    height: 16px;
    color: #6b7280;
  }

  .controls {
    display: flex;
    gap: 1rem;
    padding: 1rem 1.5rem;
    background: white;
    border-bottom: 1px solid #e5e7eb;
  }

  .search-box {
    flex: 1;
    position: relative;
  }

  .search-icon {
    position: absolute;
    left: 12px;
    top: 50%;
    transform: translateY(-50%);
    width: 16px;
    height: 16px;
    color: #9ca3af;
  }

  .search-input {
    width: 100%;
    padding: 0.5rem 0.75rem 0.5rem 2.5rem;
    border: 1px solid #d1d5db;
    border-radius: 8px;
    font-size: 0.875rem;
  }

  .search-input:focus {
    outline: none;
    border-color: #3b82f6;
    box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.1);
  }

  .filter-type {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.875rem;
    color: #6b7280;
  }

  .filter-type select {
    padding: 0.5rem;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    font-size: 0.875rem;
  }

  /* Loading and empty states */
  .loading-state,
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    padding: 3rem 1rem;
    color: #6b7280;
  }

  .loading-state .spinner {
    width: 32px;
    height: 32px;
    border: 3px solid #e5e7eb;
    border-top-color: #3b82f6;
    border-radius: 50%;
    animation: spin 1s linear infinite;
    margin-bottom: 1rem;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .empty-state svg {
    width: 64px;
    height: 64px;
    margin-bottom: 1rem;
    color: #d1d5db;
  }

  .empty-state h4 {
    margin: 0 0 0.5rem;
    font-size: 1.125rem;
    font-weight: 600;
    color: #374151;
  }

  .empty-state p {
    margin: 0;
    font-size: 0.875rem;
  }

  /* File list */
  .file-list {
    overflow-x: auto;
  }

  .file-header {
    display: grid;
    grid-template-columns: 2fr 1fr 2fr 1fr 1.5fr 1fr;
    gap: 1rem;
    padding: 0.75rem 1.5rem;
    background: #f9fafb;
    border-bottom: 1px solid #e5e7eb;
    font-size: 0.875rem;
    font-weight: 600;
    color: #374151;
  }

  .sort-btn {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: none;
    border: none;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 600;
    color: #374151;
    text-align: left;
    padding: 0;
  }

  .sort-btn:hover {
    color: #111827;
  }

  .sort-btn.active {
    color: #3b82f6;
  }

  .sort-indicator {
    font-size: 0.75rem;
  }

  .header-cell {
    display: flex;
    align-items: center;
  }

  .file-row {
    display: grid;
    grid-template-columns: 2fr 1fr 2fr 1fr 1.5fr 1fr;
    gap: 1rem;
    padding: 1rem 1.5rem;
    border-bottom: 1px solid #f3f4f6;
    cursor: pointer;
    transition: background-color 0.2s;
  }

  .file-row:hover {
    background: #f9fafb;
  }

  .file-row.selected {
    background: #eff6ff;
    border-color: #bfdbfe;
  }

  .file-row:last-child {
    border-bottom: none;
  }

  .file-name {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .file-icon {
    font-size: 1.25rem;
    flex-shrink: 0;
  }

  .name-info {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }

  .filename {
    font-weight: 500;
    color: #111827;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .file-id {
    font-size: 0.75rem;
    color: #6b7280;
    font-family: monospace;
  }

  .file-type {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .type-label {
    font-size: 0.875rem;
    color: #374151;
  }

  .format {
    font-size: 0.75rem;
    color: #6b7280;
    font-weight: 500;
  }

  .protein-info {
    display: flex;
    align-items: center;
  }

  .protein-stats {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .stat {
    font-size: 0.75rem;
    color: #6b7280;
  }

  .stat strong {
    color: #374151;
  }

  .pdb-id {
    font-size: 0.75rem;
    color: #3b82f6;
    font-weight: 500;
  }

  .no-info {
    font-size: 0.875rem;
    color: #9ca3af;
    font-style: italic;
  }

  .file-size {
    font-size: 0.875rem;
    color: #6b7280;
    font-weight: 500;
  }

  .upload-date {
    font-size: 0.875rem;
    color: #6b7280;
  }

  .file-actions {
    display: flex;
    gap: 0.5rem;
    justify-content: flex-end;
  }

  .action-btn {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 32px;
    height: 32px;
    background: white;
    border: 1px solid #d1d5db;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.2s;
  }

  .action-btn svg {
    width: 14px;
    height: 14px;
  }

  .action-btn.download {
    color: #3b82f6;
  }

  .action-btn.download:hover {
    background: #eff6ff;
    border-color: #3b82f6;
  }

  .action-btn.delete {
    color: #ef4444;
  }

  .action-btn.delete:hover {
    background: #fef2f2;
    border-color: #ef4444;
  }

  /* Responsive design */
  @media (max-width: 768px) {
    .file-header,
    .file-row {
      grid-template-columns: 2fr 1fr 1fr;
      gap: 0.5rem;
    }

    .protein-info,
    .upload-date,
    .file-size {
      display: none;
    }

    .controls {
      flex-direction: column;
      gap: 0.75rem;
    }
  }

  /* Dark mode support */
  @media (prefers-color-scheme: dark) {
    .file-manager {
      background: #1f2937;
      border-color: #374151;
    }

    .manager-header {
      background: #111827;
      border-bottom-color: #374151;
    }

    .manager-header h3 {
      color: #e5e7eb;
    }

    .controls {
      background: #1f2937;
      border-bottom-color: #374151;
    }

    .file-header {
      background: #111827;
      border-bottom-color: #374151;
      color: #d1d5db;
    }

    .file-row {
      border-bottom-color: #374151;
    }

    .file-row:hover {
      background: #374151;
    }

    .file-row.selected {
      background: #1e3a8a;
    }

    .filename {
      color: #e5e7eb;
    }
  }
</style>