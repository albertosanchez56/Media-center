import { TestBed } from '@angular/core/testing';
import { NowPlayingPanelComponent } from './now-playing-panel.component';

describe('NowPlayingPanelComponent', () => {
  it('should show the empty state while nothing is playing', async () => {
    await TestBed.configureTestingModule({ imports: [NowPlayingPanelComponent] }).compileComponents();
    const fixture = TestBed.createComponent(NowPlayingPanelComponent);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    expect(element.querySelector('.empty-state')?.textContent).toContain('Sin reproducción');
    expect(element.querySelector('button')).toBeNull();
  });
});
