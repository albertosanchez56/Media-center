import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { BridgeDiagnosticsComponent } from './diagnostics/bridge-diagnostics/bridge-diagnostics.component';
import { LibraryViewComponent } from './layout/library-view/library-view.component';
import { NowPlayingPanelComponent } from './layout/now-playing-panel/now-playing-panel.component';
import { PlayerBarComponent } from './layout/player-bar/player-bar.component';
import { SidebarComponent } from './layout/sidebar/sidebar.component';
import { NavigationService } from './navigation/navigation.service';

@Component({
  selector: 'app-root',
  imports: [
    SidebarComponent,
    LibraryViewComponent,
    NowPlayingPanelComponent,
    PlayerBarComponent,
    BridgeDiagnosticsComponent,
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {
  private readonly navigation = inject(NavigationService);

  protected readonly section = this.navigation.section;
  protected readonly currentLibrary = this.navigation.currentLibrary;
}
