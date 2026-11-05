import { TestBed } from '@angular/core/testing';
import { PlayerBarComponent } from './player-bar.component';

describe('PlayerBarComponent', () => {
  it('should show every control as inactive while there is no player', async () => {
    await TestBed.configureTestingModule({ imports: [PlayerBarComponent] }).compileComponents();
    const fixture = TestBed.createComponent(PlayerBarComponent);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    const controls = Array.from(element.querySelectorAll<HTMLButtonElement | HTMLInputElement>('button, input'));
    expect(controls.length).toBe(5);
    expect(controls.every((control) => control.disabled)).toBe(true);
    expect(controls.every((control) => control.getAttribute('aria-label'))).toBe(true);
    expect(element.textContent).toContain('Nada en reproducción');
  });
});
