import { ChangeDetectionStrategy, Component } from '@angular/core';
import { BridgeDiagnosticsComponent } from '../bridge-diagnostics/bridge-diagnostics.component';
import { EngineDiagnosticsComponent } from '../engine-diagnostics/engine-diagnostics.component';

@Component({
  selector: 'app-diagnostics-view',
  imports: [BridgeDiagnosticsComponent, EngineDiagnosticsComponent],
  templateUrl: './diagnostics-view.component.html',
  styleUrl: './diagnostics-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DiagnosticsViewComponent {}
