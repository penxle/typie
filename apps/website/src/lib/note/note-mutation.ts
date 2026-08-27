import { createMutation } from '@mearie/svelte';
import * as Sentry from '@sentry/sveltekit';
import { TypieError } from '@typie/lib/errors';
import { createStableContext } from '@typie/ui/context/stable';
import { unwrapError } from '$lib/graphql/error';
import { graphql } from '$mearie';
import type { VariablesOf } from '@mearie/svelte';
import type { NoteStatus } from '$mearie';
import type { NoteSync, NoteUpdate } from './note-sync.svelte';

const createNoteDocument = graphql(`
  mutation NoteOperations_CreateNote_Mutation($input: CreateNoteInput!) {
    createNote(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);
const updateNoteDocument = graphql(`
  mutation NoteOperations_UpdateNote_Mutation($input: UpdateNoteInput!) {
    updateNote(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);
const moveNoteDocument = graphql(`
  mutation NoteOperations_MoveNote_Mutation($input: MoveNoteInput!) {
    moveNote(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);
const deleteNoteDocument = graphql(`
  mutation NoteOperations_DeleteNote_Mutation($input: DeleteNoteInput!) {
    deleteNote(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);
const addNoteEntityDocument = graphql(`
  mutation NoteOperations_AddNoteEntity_Mutation($input: AddNoteEntityInput!) {
    addNoteEntity(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);
const removeNoteEntityDocument = graphql(`
  mutation NoteOperations_RemoveNoteEntity_Mutation($input: RemoveNoteEntityInput!) {
    removeNoteEntity(input: $input) {
      id
      content
      color
      order
      status
      updatedAt
      site {
        id
      }
    }
  }
`);

type AddNoteEntityInput = VariablesOf<typeof addNoteEntityDocument>['input'];
type CreateNoteInput = VariablesOf<typeof createNoteDocument>['input'];
type DeleteNoteInput = VariablesOf<typeof deleteNoteDocument>['input'];
type MoveNoteInput = VariablesOf<typeof moveNoteDocument>['input'];
type RemoveNoteEntityInput = VariablesOf<typeof removeNoteEntityDocument>['input'];
type UpdateNoteInput = VariablesOf<typeof updateNoteDocument>['input'];

type NoteSnapshot = {
  id: string;
  content: string;
  color: string;
  order: string;
  status: NoteStatus;
  updatedAt: string;
  site: { id: string };
};

type NoteMutationTarget = {
  siteId: string;
  noteId: string;
};

type NoteMutationAnalytics = {
  onSuccess?: (value: NoteSnapshot) => void;
  onTerminal?: (target: NoteMutationTarget) => void;
};

type NoteMutationOutcome =
  | { status: 'success'; value: NoteSnapshot }
  | { status: 'subscription_gated' }
  | { status: 'not_found' }
  | { status: 'failure'; error: unknown };

type NoteMutationOptions = {
  lastKnown?: NoteMutationTarget;
  analytics?: NoteMutationAnalytics;
};

type NoteCreateOptions = Pick<NoteMutationOptions, 'analytics'>;

type NoteAdmissionVia = 'notes_create' | 'notes_update' | 'notes_move' | 'notes_add_entity' | 'notes_remove_entity';

type NoteOperationsOptions = {
  clientId: string;
  sync: NoteSync;
  admit: (via: NoteAdmissionVia) => boolean;
};

type NoteMutationInput = { clientId?: string | null };
type NoteMutationKind = NoteUpdate['kind'];

function classifyNoteMutationError(error: unknown): 'not_found' | 'failure' {
  const unwrapped = unwrapError(error);
  return unwrapped instanceof TypieError && unwrapped.code === 'not_found' ? 'not_found' : 'failure';
}

function reportNoteMutationFailure(error: unknown): void {
  try {
    Sentry.captureException(error);
  } catch {
    // Error reporting must not change the mutation outcome.
  }
}

export class NoteOperations {
  readonly #clientId: string;
  readonly #sync: NoteSync;
  readonly #admit: NoteOperationsOptions['admit'];
  readonly #create: (input: CreateNoteInput) => Promise<NoteSnapshot>;
  readonly #update: (input: UpdateNoteInput) => Promise<NoteSnapshot>;
  readonly #move: (input: MoveNoteInput) => Promise<NoteSnapshot>;
  readonly #delete: (input: DeleteNoteInput) => Promise<NoteSnapshot>;
  readonly #addEntity: (input: AddNoteEntityInput) => Promise<NoteSnapshot>;
  readonly #removeEntity: (input: RemoveNoteEntityInput) => Promise<NoteSnapshot>;

  constructor({ clientId, sync, admit }: NoteOperationsOptions) {
    const [createNote] = createMutation(createNoteDocument);
    const [updateNote] = createMutation(updateNoteDocument);
    const [moveNote] = createMutation(moveNoteDocument);
    const [deleteNote] = createMutation(deleteNoteDocument);
    const [addNoteEntity] = createMutation(addNoteEntityDocument);
    const [removeNoteEntity] = createMutation(removeNoteEntityDocument);

    this.#clientId = clientId;
    this.#sync = sync;
    this.#admit = admit;
    this.#create = async (input) => {
      const data = await createNote({ input });
      return data.createNote;
    };
    this.#update = async (input) => {
      const data = await updateNote({ input });
      return data.updateNote;
    };
    this.#move = async (input) => {
      const data = await moveNote({ input });
      return data.moveNote;
    };
    this.#delete = async (input) => {
      const data = await deleteNote({ input });
      return data.deleteNote;
    };
    this.#addEntity = async (input) => {
      const data = await addNoteEntity({ input });
      return data.addNoteEntity;
    };
    this.#removeEntity = async (input) => {
      const data = await removeNoteEntity({ input });
      return data.removeNoteEntity;
    };
  }

  async #execute<TInput extends NoteMutationInput>({
    input,
    mutate,
    kind,
    via,
    options,
    notFoundIsFailure = false,
    publishLocal = true,
  }: {
    input: Omit<TInput, 'clientId'>;
    mutate: (input: TInput) => Promise<NoteSnapshot>;
    kind: NoteMutationKind;
    via?: NoteAdmissionVia;
    options: NoteMutationOptions;
    notFoundIsFailure?: boolean;
    publishLocal?: boolean;
  }): Promise<NoteMutationOutcome> {
    if (via) {
      try {
        if (!this.#admit(via)) {
          return { status: 'subscription_gated' };
        }
      } catch (err) {
        reportNoteMutationFailure(err);
        return { status: 'failure', error: err };
      }
    }

    let value: NoteSnapshot;
    try {
      value = await mutate({ ...input, clientId: this.#clientId } as TInput);
    } catch (err) {
      if (notFoundIsFailure || classifyNoteMutationError(err) !== 'not_found') {
        reportNoteMutationFailure(err);
        return { status: 'failure', error: err };
      }

      if (options.lastKnown) {
        let terminalApplied = false;
        try {
          terminalApplied = this.#sync.markNotFound(options.lastKnown);
        } catch (err_) {
          reportNoteMutationFailure(err_);
        }
        if (terminalApplied) {
          try {
            options.analytics?.onTerminal?.(options.lastKnown);
          } catch {
            // Analytics is best-effort after the terminal state has been applied.
          }
        }
      }
      return { status: 'not_found' };
    }

    if (publishLocal) {
      try {
        this.#sync.publishLocal({
          kind,
          noteId: value.id,
          siteId: value.site.id,
        });
      } catch (err) {
        reportNoteMutationFailure(err);
      }
    }
    try {
      options.analytics?.onSuccess?.(value);
    } catch {
      // Analytics is best-effort after the successful mutation has been published.
    }
    return { status: 'success', value };
  }

  create(input: Omit<CreateNoteInput, 'clientId'>, options: NoteCreateOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({
      input,
      mutate: this.#create,
      kind: 'CREATED',
      via: 'notes_create',
      options,
      notFoundIsFailure: true,
    });
  }

  update(input: Omit<UpdateNoteInput, 'clientId'>, options: NoteMutationOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({ input, mutate: this.#update, kind: 'UPDATED', via: 'notes_update', options });
  }

  move(input: Omit<MoveNoteInput, 'clientId'>, options: NoteMutationOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({
      input,
      mutate: this.#move,
      kind: 'UPDATED',
      via: 'notes_move',
      options,
      publishLocal: false,
    });
  }

  delete(input: Omit<DeleteNoteInput, 'clientId'>, options: NoteMutationOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({ input, mutate: this.#delete, kind: 'DELETED', options });
  }

  addEntity(input: Omit<AddNoteEntityInput, 'clientId'>, options: NoteMutationOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({ input, mutate: this.#addEntity, kind: 'UPDATED', via: 'notes_add_entity', options });
  }

  removeEntity(input: Omit<RemoveNoteEntityInput, 'clientId'>, options: NoteMutationOptions = {}): Promise<NoteMutationOutcome> {
    return this.#execute({ input, mutate: this.#removeEntity, kind: 'UPDATED', via: 'notes_remove_entity', options });
  }
}

const [getNoteOperationsContext, setNoteOperationsContext] = createStableContext<NoteOperations>('note.NoteOperations');

export { getNoteOperationsContext, setNoteOperationsContext };
