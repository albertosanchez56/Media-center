import { TestBed } from '@angular/core/testing';
import { FileDialog } from '../../import/file-dialog';
import { LIBRARIES } from '../../navigation/navigation';
import { LibraryViewComponent } from './library-view.component';

class FakeDialog extends FileDialog {
  filesResult: string | string[] | null = null;

  openFiles(): Promise<string | string[] | null> {
    return Promise.resolve(this.filesResult);
  }

  openFolder(): Promise<string | string[] | null> {
    return Promise.resolve(null);
  }
}

describe('LibraryViewComponent', () => {
  const dialog = new FakeDialog();

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LibraryViewComponent],
      providers: [{ provide: FileDialog, useValue: dialog }],
    }).compileComponents();
  });

  it('should show the title, empty state and import button without a panel', async () => {
    const fixture = TestBed.createComponent(LibraryViewComponent);
    const movies = LIBRARIES.find((library) => library.id === 'movie')!;
    fixture.componentRef.setInput('library', movies);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    expect(element.querySelector('h1')?.textContent).toContain('Películas');
    expect(element.querySelector('.library-content')?.textContent).toContain(movies.emptyTitle);
    expect(element.querySelector('.import-button')?.textContent).toContain('Importar');
    expect(element.querySelector('.import-panel')).toBeNull();
  });

  it('should keep a long mp4 path readable and the chosen library after changing section', async () => {
    const fixture = TestBed.createComponent(LibraryViewComponent);
    const music = LIBRARIES.find((library) => library.id === 'music')!;
    const videos = LIBRARIES.find((library) => library.id === 'video')!;
    const long = `C:\\${'carpeta larga\\'.repeat(8)}clip.mp4`;
    dialog.filesResult = long;
    fixture.componentRef.setInput('library', music);
    await fixture.whenStable();

    const element = fixture.nativeElement as HTMLElement;
    element.querySelector<HTMLButtonElement>('.import-button')!.click();
    await fixture.whenStable();
    const chooseFiles = [...element.querySelectorAll('button')].find((button) =>
      button.textContent?.includes('Elegir archivos'),
    );
    chooseFiles!.click();
    await fixture.whenStable();

    expect(element.querySelector('.path-name')?.textContent).toBe('clip.mp4');
    expect(element.querySelector('.path-full')?.getAttribute('title')).toBe(long);
    expect(element.querySelector('.summary')?.textContent).toContain('Música');

    element.querySelector<HTMLInputElement>('input[value="movie"]')!.dispatchEvent(new Event('change'));
    await fixture.whenStable();
    expect(element.querySelector('.summary')?.textContent).toContain('Películas');

    fixture.componentRef.setInput('library', videos);
    await fixture.whenStable();
    expect(element.querySelector('h1')?.textContent).toContain('Vídeos');
    expect(element.querySelector('.summary')?.textContent).toContain('Películas');
    expect(element.querySelector('.summary')?.textContent).toContain('clip.mp4');
  });
});
