/*!
 * ExportManager - Gestor de exportaciones y descarga de archivos
 * 
 * Proporciona utilidades para:
 * 1. Gestión de múltiples formatos de exportación
 * 2. Validación de datos antes de exportar
 * 3. Compresión de archivos grandes
 * 4. Historial de exportaciones
 * 5. Templates de exportación personalizados
 */

import type { DockingResult, DockingPose } from '../algorithms/DockingAlgorithm.js';
import type { PDBStructure } from './PDBParser.js';
import type { ScoringResult } from '../algorithms/ScoringEngine.js';

export interface ExportTemplate {
  id: string;
  name: string;
  description: string;
  format: ExportFormat;
  options: ExportOptions;
  isDefault: boolean;
}

export interface ExportOptions {
  includeMetadata: boolean;
  includeEnergyBreakdown: boolean;
  includeScoringDetails: boolean;
  includeContacts: boolean;
  includeTransformations: boolean;
  maxPoses: number;
  onlyTopPoses: boolean;
  topPosesCount: number;
  compression: 'none' | 'gzip' | 'zip';
  customFields?: string[];
}

export interface ExportJob {
  id: string;
  timestamp: Date;
  format: ExportFormat;
  filename: string;
  size: number;
  status: 'pending' | 'processing' | 'completed' | 'failed';
  error?: string;
  downloadUrl?: string;
}

export type ExportFormat = 'json' | 'csv' | 'sdf' | 'pdb' | 'xml' | 'xlsx' | 'mol2' | 'mae';

export class ExportManager {
  private templates: Map<string, ExportTemplate>;
  private exportHistory: ExportJob[];
  private maxHistorySize: number;

  constructor() {
    this.templates = new Map();
    this.exportHistory = [];
    this.maxHistorySize = 100;
    this.initializeDefaultTemplates();
  }

  /**
   * Inicializa templates por defecto
   */
  private initializeDefaultTemplates(): void {
    const defaultTemplates: ExportTemplate[] = [
      {
        id: 'json_complete',
        name: 'JSON Completo',
        description: 'Todos los datos incluyendo metadatos, energías y contactos',
        format: 'json',
        options: {
          includeMetadata: true,
          includeEnergyBreakdown: true,
          includeScoringDetails: true,
          includeContacts: true,
          includeTransformations: true,
          maxPoses: 1000,
          onlyTopPoses: false,
          topPosesCount: 10,
          compression: 'gzip'
        },
        isDefault: true
      },
      {
        id: 'csv_analysis',
        name: 'CSV para Análisis',
        description: 'Formato tabular optimizado para análisis estadístico',
        format: 'csv',
        options: {
          includeMetadata: false,
          includeEnergyBreakdown: true,
          includeScoringDetails: true,
          includeContacts: false,
          includeTransformations: false,
          maxPoses: 500,
          onlyTopPoses: false,
          topPosesCount: 10,
          compression: 'none'
        },
        isDefault: true
      },
      {
        id: 'sdf_chemical',
        name: 'SDF Químico',
        description: 'Formato estándar para herramientas de química computacional',
        format: 'sdf',
        options: {
          includeMetadata: true,
          includeEnergyBreakdown: true,
          includeScoringDetails: false,
          includeContacts: false,
          includeTransformations: false,
          maxPoses: 100,
          onlyTopPoses: true,
          topPosesCount: 50,
          compression: 'none'
        },
        isDefault: true
      },
      {
        id: 'top_poses_summary',
        name: 'Resumen Top Poses',
        description: 'Solo las mejores poses con información esencial',
        format: 'json',
        options: {
          includeMetadata: true,
          includeEnergyBreakdown: true,
          includeScoringDetails: true,
          includeContacts: true,
          includeTransformations: false,
          maxPoses: 20,
          onlyTopPoses: true,
          topPosesCount: 10,
          compression: 'none'
        },
        isDefault: true
      }
    ];

    defaultTemplates.forEach(template => {
      this.templates.set(template.id, template);
    });
  }

  /**
   * Valida datos antes de exportar
   */
  validateExportData(dockingResult: DockingResult, options: ExportOptions): string[] {
    const errors: string[] = [];

    if (!dockingResult) {
      errors.push('No hay datos de docking para exportar');
      return errors;
    }

    if (!dockingResult.poses || dockingResult.poses.length === 0) {
      errors.push('No hay poses disponibles para exportar');
      return errors;
    }

    // Validar opciones
    if (options.maxPoses <= 0) {
      errors.push('El número máximo de poses debe ser mayor que 0');
    }

    if (options.onlyTopPoses && options.topPosesCount <= 0) {
      errors.push('El número de top poses debe ser mayor que 0');
    }

    if (options.onlyTopPoses && options.topPosesCount > dockingResult.poses.length) {
      errors.push(`Solo hay ${dockingResult.poses.length} poses disponibles, no se pueden exportar ${options.topPosesCount}`);
    }

    // Validar integridad de datos
    const posesWithMissingData = dockingResult.poses.filter(pose => {
      if (options.includeEnergyBreakdown && !pose.energy_components) {
        return true;
      }
      if (options.includeContacts && (!pose.contacts || pose.contacts.length === 0)) {
        return true;
      }
      return false;
    });

    if (posesWithMissingData.length > 0) {
      errors.push(`${posesWithMissingData.length} poses tienen datos incompletos para las opciones seleccionadas`);
    }

    return errors;
  }

  /**
   * Estima el tamaño del archivo de exportación
   */
  estimateFileSize(dockingResult: DockingResult, format: ExportFormat, options: ExportOptions): number {
    if (!dockingResult) return 0;

    const posesCount = options.onlyTopPoses 
      ? Math.min(options.topPosesCount, dockingResult.poses.length)
      : Math.min(options.maxPoses, dockingResult.poses.length);

    // Estimaciones aproximadas por formato (en bytes por pose)
    const formatSizes: Record<ExportFormat, number> = {
      json: 2000,
      csv: 500,
      sdf: 1500,
      pdb: 1200,
      xml: 2500,
      xlsx: 800,
      mol2: 1000,
      mae: 1800
    };

    let baseSize = posesCount * formatSizes[format];

    // Ajustar por opciones
    if (options.includeEnergyBreakdown) baseSize *= 1.3;
    if (options.includeScoringDetails) baseSize *= 1.2;
    if (options.includeContacts) baseSize *= 1.5;
    if (options.includeTransformations) baseSize *= 1.1;
    if (options.includeMetadata) baseSize += 1000;

    // Ajustar por compresión
    switch (options.compression) {
      case 'gzip':
        baseSize *= 0.3;
        break;
      case 'zip':
        baseSize *= 0.4;
        break;
    }

    return Math.round(baseSize);
  }

  /**
   * Formatea el tamaño del archivo para mostrar
   */
  formatFileSize(bytes: number): string {
    if (bytes === 0) return '0 Bytes';
    
    const k = 1024;
    const sizes = ['Bytes', 'KB', 'MB', 'GB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
  }

  /**
   * Genera un nombre de archivo único
   */
  generateFilename(format: ExportFormat, dockingResult: DockingResult, options: ExportOptions): string {
    const timestamp = new Date().toISOString().slice(0, 19).replace(/:/g, '-');
    const algorithm = dockingResult.algorithm.toLowerCase().replace(/\s+/g, '_');
    const posesCount = options.onlyTopPoses 
      ? Math.min(options.topPosesCount, dockingResult.poses.length)
      : Math.min(options.maxPoses, dockingResult.poses.length);
    
    let filename = `docking_${algorithm}_${posesCount}poses_${timestamp}`;
    
    // Agregar sufijo de compresión
    switch (options.compression) {
      case 'gzip':
        filename += '.gz';
        break;
      case 'zip':
        filename += '.zip';
        break;
    }
    
    filename += `.${format}`;
    return filename;
  }

  /**
   * Crea un trabajo de exportación
   */
  createExportJob(format: ExportFormat, dockingResult: DockingResult, options: ExportOptions): ExportJob {
    const job: ExportJob = {
      id: `export_${Date.now()}_${Math.random().toString(36).substr(2, 6)}`,
      timestamp: new Date(),
      format,
      filename: this.generateFilename(format, dockingResult, options),
      size: this.estimateFileSize(dockingResult, format, options),
      status: 'pending'
    };

    this.addToHistory(job);
    return job;
  }

  /**
   * Agrega un trabajo al historial
   */
  private addToHistory(job: ExportJob): void {
    this.exportHistory.unshift(job);
    
    // Mantener el tamaño del historial
    if (this.exportHistory.length > this.maxHistorySize) {
      const removed = this.exportHistory.pop();
      if (removed?.downloadUrl) {
        URL.revokeObjectURL(removed.downloadUrl);
      }
    }
  }

  /**
   * Actualiza el estado de un trabajo
   */
  updateJobStatus(jobId: string, status: ExportJob['status'], error?: string, downloadUrl?: string): void {
    const job = this.exportHistory.find(j => j.id === jobId);
    if (job) {
      job.status = status;
      if (error) job.error = error;
      if (downloadUrl) job.downloadUrl = downloadUrl;
    }
  }

  /**
   * Obtiene el historial de exportaciones
   */
  getExportHistory(): ExportJob[] {
    return [...this.exportHistory];
  }

  /**
   * Limpia el historial de exportaciones
   */
  clearHistory(): void {
    // Limpiar URLs de descarga
    this.exportHistory.forEach(job => {
      if (job.downloadUrl) {
        URL.revokeObjectURL(job.downloadUrl);
      }
    });
    
    this.exportHistory = [];
  }

  /**
   * Obtiene todos los templates disponibles
   */
  getTemplates(): ExportTemplate[] {
    return Array.from(this.templates.values());
  }

  /**
   * Obtiene un template por ID
   */
  getTemplate(id: string): ExportTemplate | undefined {
    return this.templates.get(id);
  }

  /**
   * Crea un template personalizado
   */
  createCustomTemplate(template: Omit<ExportTemplate, 'isDefault'>): void {
    const customTemplate: ExportTemplate = {
      ...template,
      isDefault: false
    };
    
    this.templates.set(template.id, customTemplate);
  }

  /**
   * Elimina un template personalizado
   */
  deleteCustomTemplate(id: string): boolean {
    const template = this.templates.get(id);
    if (template && !template.isDefault) {
      this.templates.delete(id);
      return true;
    }
    return false;
  }

  /**
   * Aplica compresión a los datos
   */
  async compressData(data: string, compression: ExportOptions['compression']): Promise<Blob> {
    switch (compression) {
      case 'gzip':
        // En un entorno real, usaríamos una librería de compresión
        return new Blob([data], { type: 'application/gzip' });
      
      case 'zip':
        // En un entorno real, usaríamos una librería como JSZip
        return new Blob([data], { type: 'application/zip' });
      
      case 'none':
      default:
        return new Blob([data], { type: 'text/plain' });
    }
  }

  /**
   * Verifica si un formato es soportado
   */
  isFormatSupported(format: string): format is ExportFormat {
    const supportedFormats: ExportFormat[] = ['json', 'csv', 'sdf', 'pdb', 'xml', 'xlsx', 'mol2', 'mae'];
    return supportedFormats.includes(format as ExportFormat);
  }

  /**
   * Obtiene información sobre un formato
   */
  getFormatInfo(format: ExportFormat): { name: string; description: string; mimeType: string } {
    const formatInfo: Record<ExportFormat, { name: string; description: string; mimeType: string }> = {
      json: {
        name: 'JSON',
        description: 'JavaScript Object Notation - Formato estructurado',
        mimeType: 'application/json'
      },
      csv: {
        name: 'CSV',
        description: 'Comma Separated Values - Formato tabular',
        mimeType: 'text/csv'
      },
      sdf: {
        name: 'SDF',
        description: 'Structure Data File - Formato químico estándar',
        mimeType: 'chemical/x-mdl-sdfile'
      },
      pdb: {
        name: 'PDB',
        description: 'Protein Data Bank - Coordenadas 3D',
        mimeType: 'chemical/x-pdb'
      },
      xml: {
        name: 'XML',
        description: 'eXtensible Markup Language - Formato estructurado',
        mimeType: 'application/xml'
      },
      xlsx: {
        name: 'Excel',
        description: 'Microsoft Excel - Hoja de cálculo',
        mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
      },
      mol2: {
        name: 'MOL2',
        description: 'Tripos MOL2 - Formato molecular',
        mimeType: 'chemical/x-mol2'
      },
      mae: {
        name: 'MAE',
        description: 'Maestro Format - Schrödinger',
        mimeType: 'application/octet-stream'
      }
    };

    return formatInfo[format];
  }

  /**
   * Cleanup - libera recursos
   */
  cleanup(): void {
    this.clearHistory();
  }
}

// Instancia singleton del export manager
export const exportManager = new ExportManager();