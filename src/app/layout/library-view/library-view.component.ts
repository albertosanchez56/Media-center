import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { Library } from '../../navigation/navigation';
import { EmptyStateComponent } from '../../shared/empty-state/empty-state.component';
import { IconComponent } from '../../shared/icon/icon.component';

@Component({
  selector: 'app-library-view',
  imports: [IconComponent, EmptyStateComponent],
  templateUrl: './library-view.component.html',
  styleUrl: './library-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LibraryViewComponent {
  readonly library = input.required<Library>();
}
