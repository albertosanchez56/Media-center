import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { LIBRARIES, LibraryId } from '../navigation/navigation';
import { pathName } from './import-selection';
import { ImportSession } from './import-session';

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
}
