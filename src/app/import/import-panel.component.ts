import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LIBRARIES, LibraryId } from '../navigation/navigation';
import { pathName } from './import-selection';
import { ImportSession } from './import-session';
import { ImportItem } from './import-transport';

@Component({
  selector: 'app-import-panel',
  templateUrl: './import-panel.component.html',
  styleUrl: './import-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ImportPanelComponent {
  protected readonly session = inject(ImportSession);
  protected readonly libraries = LIBRARIES;
  protected readonly pathName = pathName;

  protected libraryName(id: LibraryId): string {
    return this.libraries.find((library) => library.id === id)?.name ?? id;
  }

  protected canImport(): boolean {
    const folder = this.session.folder();
    return (
      this.session.selection() !== null &&
      !this.session.pending() &&
      !this.session.running() &&
      folder !== null &&
      folder.state !== 'unavailable' &&
      folder.state !== 'missing'
    );
  }

  protected folderLabel(): string {
    const folder = this.session.folder();
    if (!folder || folder.state === 'missing') {
      return 'Sin carpeta propuesta';
    }
    const state =
      folder.state === 'confirmed'
        ? 'confirmada'
        : folder.state === 'unavailable'
          ? 'no disponible'
          : 'propuesta';
    return `${folder.path} (${state})`;
  }

  protected hasSavedCard(): boolean {
    return this.session.results().some((item) => item.state === 'completed');
  }

  protected stateLabel(item: ImportItem): string {
    switch (item.state) {
      case 'completed':
        return 'Completado';
      case 'failed':
        return 'No completado';
      case 'needs_review':
        return 'Necesita revisión';
      case 'skipped':
        return 'Omitido';
      case 'ready_for_db':
        return 'Pendiente de registro';
      case 'transferring':
        return 'Copiando';
      case 'verifying':
        return 'Comprobando';
      default:
        return item.state;
    }
  }
}
