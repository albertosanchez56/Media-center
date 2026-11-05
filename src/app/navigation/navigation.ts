import { IconName } from '../shared/icon/icon.component';

export type LibraryId = 'music' | 'video' | 'movie';

export type Section = LibraryId | 'diagnostics';

export interface Library {
  id: LibraryId;
  name: string;
  icon: IconName;
  emptyTitle: string;
  emptyText: string;
}

export const LIBRARIES: readonly Library[] = [
  {
    id: 'music',
    name: 'Música',
    icon: 'music',
    emptyTitle: 'Todavía no hay música en tu biblioteca',
    emptyText: 'Las canciones aparecerán aquí cuando la importación esté disponible.',
  },
  {
    id: 'video',
    name: 'Vídeos',
    icon: 'video',
    emptyTitle: 'Todavía no hay vídeos en tu biblioteca',
    emptyText: 'Los vídeos aparecerán aquí cuando la importación esté disponible.',
  },
  {
    id: 'movie',
    name: 'Películas',
    icon: 'movie',
    emptyTitle: 'Todavía no hay películas en tu biblioteca',
    emptyText: 'Las películas aparecerán aquí cuando la importación esté disponible.',
  },
];
