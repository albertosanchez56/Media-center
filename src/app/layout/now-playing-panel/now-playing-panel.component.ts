import { ChangeDetectionStrategy, Component } from '@angular/core';
import { EmptyStateComponent } from '../../shared/empty-state/empty-state.component';

@Component({
  selector: 'app-now-playing-panel',
  imports: [EmptyStateComponent],
  templateUrl: './now-playing-panel.component.html',
  styleUrl: './now-playing-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NowPlayingPanelComponent {}
