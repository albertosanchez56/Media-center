import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NavigationService } from '../../navigation/navigation.service';
import { SidebarComponent } from './sidebar.component';

describe('SidebarComponent', () => {
  let fixture: ComponentFixture<SidebarComponent>;
  let element: HTMLElement;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [SidebarComponent] }).compileComponents();
    fixture = TestBed.createComponent(SidebarComponent);
    await fixture.whenStable();
    element = fixture.nativeElement as HTMLElement;
  });

  function libraryButtons(): HTMLButtonElement[] {
    return Array.from(
      element.querySelectorAll<HTMLButtonElement>('nav[aria-label="Bibliotecas"] button'),
    );
  }

  it('should list the three libraries with accessible names', () => {
    const names = libraryButtons().map((button) => button.textContent?.trim());
    expect(names).toEqual(['Música', 'Vídeos', 'Películas']);
  });

  it('should mark the selected library as current', async () => {
    const [music, video] = libraryButtons();
    expect(music.getAttribute('aria-current')).toBe('page');

    video.click();
    await fixture.whenStable();

    expect(TestBed.inject(NavigationService).section()).toBe('video');
    expect(video.getAttribute('aria-current')).toBe('page');
    expect(music.getAttribute('aria-current')).toBeNull();
  });

  it('should show an empty playlists section without a create action', () => {
    const playlists = element.querySelector('.playlists') as HTMLElement;
    expect(playlists.textContent).toContain('Todavía no hay playlists');
    expect(playlists.querySelector('button')).toBeNull();
  });

  it('should offer the diagnostics entry in development mode', () => {
    expect(element.querySelector('.diagnostics')?.textContent).toContain('Diagnóstico');
  });
});
