import { CognitoIdentityClient } from '@aws-sdk/client-cognito-identity';
import { InvokeCommand, LambdaClient } from '@aws-sdk/client-lambda';
import { fromCognitoIdentityPool } from '@aws-sdk/credential-providers';
import { UserManager } from 'oidc-client-ts';

import type { RuntimeConfig } from '@/runtime-config';

// ── Row types (mirror rat-core/src/queries.rs) ───────────────────────

export type RepoRow = {
  repo_id: string;
  branch: string;
  indexed_commit_id: string | null;
  description: string | null;
  file_count: number;
  snippet_count: number;
};

export type SnippetRow = {
  id: number;
  repo_id: string;
  source_path: string;
  content: string;
  description: string;
  source_type: string;
  symbol_name: string | null;
  start_line: number | null;
  end_line: number | null;
  language: string | null;
};

export type FileRow = {
  id: number;
  repo_id: string;
  source_path: string;
  content: string;
  language: string | null;
};

export type FileListRow = {
  id: number;
  source_path: string;
  language: string | null;
};

// ── Request/Response shapes (mirror rat-core/src/api.rs) ─────────────

export type SearchRequest = {
  action: 'search';
  query: string;
  repo_id?: string;
  source_type?: string;
  limit?: number;
};
export type SearchResult = SnippetRow & { score: number };
export type SearchResponse = { results: SearchResult[] };

export type ListRequest = { action: 'list' };
export type ListResponse = { repos: RepoRow[] };

export type RepoSearchRequest = {
  action: 'repo_search';
  query: string;
  limit?: number;
};
export type RepoSearchResult = RepoRow & { score: number };
export type RepoSearchResponse = { results: RepoSearchResult[] };

export type RepoGetRequest = { action: 'repo_get'; repo_id: string };
export type RepoGetResponse = { repo: RepoRow | null };

export type FileGetRequest = {
  action: 'file_get';
  repo_id: string;
  source_path: string;
};
export type FileGetResponse = { file: FileRow | null };

export type FileListRequest = { action: 'file_list'; repo_id: string };
export type FileListResponse = { files: FileListRow[] };

export type SnippetListRequest = {
  action: 'snippet_list';
  repo_id: string;
  source_path: string;
};
export type SnippetListResponse = { snippets: SnippetRow[] };

export type PurgeRequest = { action: 'purge'; repo_id: string };
export type PurgeResponse = {
  repo_id: string;
  found: boolean;
  deleted_files: number;
  deleted_snippets: number;
};

export type RepoUpsertRequest = {
  action: 'repo_upsert';
  repo_id: string;
  branch: string;
  commit_id?: string;
  readme?: string;
};
export type RepoUpsertResponse = { repo_id: string };

export type ApiRequest =
  | SearchRequest
  | ListRequest
  | RepoSearchRequest
  | RepoGetRequest
  | FileGetRequest
  | FileListRequest
  | SnippetListRequest
  | PurgeRequest
  | RepoUpsertRequest;

// ── Client ────────────────────────────────────────────────────────────

let lambda: LambdaClient | null = null;
let functionArn: string | null = null;

export function configureRatApi(
  rc: RuntimeConfig,
  userManager: UserManager,
): void {
  const region = rc.cognito.region;
  const provider = `cognito-idp.${region}.amazonaws.com/${rc.cognito.userPoolId}`;

  lambda = new LambdaClient({
    region,
    credentials: fromCognitoIdentityPool({
      client: new CognitoIdentityClient({ region }),
      identityPoolId: rc.cognito.identityPoolId,
      logins: {
        [provider]: async () => {
          const user = await userManager.getUser();
          if (!user || user.expired) {
            const renewed = await userManager.signinSilent();
            if (!renewed?.id_token) {
              throw new Error('No Cognito ID token available — sign in first.');
            }
            return renewed.id_token;
          }
          return user.id_token!;
        },
      },
    }),
  });
  functionArn = rc.api.functionArn;
}

async function invoke<R>(req: ApiRequest): Promise<R> {
  if (!lambda || !functionArn) {
    throw new Error('rat-api not configured. Call configureRatApi() first.');
  }

  const result = await lambda.send(
    new InvokeCommand({
      FunctionName: functionArn,
      Payload: new TextEncoder().encode(JSON.stringify(req)),
    }),
  );

  if (result.FunctionError) {
    const body = result.Payload
      ? new TextDecoder().decode(result.Payload)
      : '(no payload)';
    throw new Error(`Lambda ${result.FunctionError}: ${body}`);
  }

  if (!result.Payload) {
    throw new Error('Lambda returned empty payload');
  }
  return JSON.parse(new TextDecoder().decode(result.Payload)) as R;
}

// ── Convenience wrappers ──────────────────────────────────────────────

export function search(
  query: string,
  opts?: { repo_id?: string; source_type?: string; limit?: number },
): Promise<SearchResponse> {
  return invoke<SearchResponse>({ action: 'search', query, ...opts });
}

export function listRepos(): Promise<ListResponse> {
  return invoke<ListResponse>({ action: 'list' });
}

export function searchRepos(
  query: string,
  limit?: number,
): Promise<RepoSearchResponse> {
  return invoke<RepoSearchResponse>({ action: 'repo_search', query, limit });
}

export function getRepo(repo_id: string): Promise<RepoGetResponse> {
  return invoke<RepoGetResponse>({ action: 'repo_get', repo_id });
}

export function getFile(
  repo_id: string,
  source_path: string,
): Promise<FileGetResponse> {
  return invoke<FileGetResponse>({
    action: 'file_get',
    repo_id,
    source_path,
  });
}

export function listFiles(repo_id: string): Promise<FileListResponse> {
  return invoke<FileListResponse>({ action: 'file_list', repo_id });
}

export function listSnippetsByFile(
  repo_id: string,
  source_path: string,
): Promise<SnippetListResponse> {
  return invoke<SnippetListResponse>({
    action: 'snippet_list',
    repo_id,
    source_path,
  });
}
