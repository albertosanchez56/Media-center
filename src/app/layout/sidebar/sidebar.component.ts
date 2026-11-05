import { ChangeDetectionStrategy, Component, inject, isDevMode } from '@angular/core';
import { LIBRARIES, Section } from '../../navigation/navigation';
import { NavigationService } from '../../navigation/navigation.service';
import { EmptyStateComponent } from '../../shared/empty-state/empty-state.component';
import { IconComponent } from '../../shared/icon/icon.component';

@Component({
  selector: 'app-sidebar',
  imports: [IconComponent, EmptyStateComponent],
  templateUrl: './sidebar.component.html',
  styleUrl: './sidebar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SidebarComponent {
  private readonly navigation = inject(NavigationService);

  protected readonly libraries = LIBRARIES;
  protected readonly section = this.navigation.section;
  protected readonly showDiagnostics = isDevMode();

  protected select(section: Section): void {
    this.navigation.select(section);
  }
}
