import { ChangeDetectionStrategy, Component, inject, input } from '@angular/core';
import { ImportPanelComponent } from '../../import/import-panel.component';
import { ImportSession } from '../../import/import-session';
import { Library } from '../../navigation/navigation';
import { EmptyStateComponent } from '../../shared/empty-state/empty-state.component';
import { IconComponent } from '../../shared/icon/icon.component';

@Component({
  selector: 'app-library-view',
  imports: [IconComponent, EmptyStateComponent, ImportPanelComponent],
  templateUrl: './library-view.component.html',
  styleUrl: './library-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LibraryViewComponent {
  private readonly session = inject(ImportSession);

  readonly library = input.required<Library>();

  protected openImport(): void {
    this.session.show(this.library().id);
  }
}
