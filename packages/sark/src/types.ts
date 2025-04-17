import type graphql from 'graphql';

export type DistributiveOmit<T, K extends string> = T extends unknown ? Omit<T, K> : never;
export type IsEmpty<T> = T extends { [K in keyof T]: never } ? true : false;

export type ContextHolder = {
  context: Context | null;
};

export type Context = {
  projectDir: string;
  sarkDir: string;

  schema: graphql.GraphQLSchema;
  artifacts: Artifact[];
};

export type ArtifactKind = 'query' | 'mutation' | 'subscription' | 'fragment';

export type ArtifactBase = {
  name: string;
  file: string;
  source: string;
  selections: Selection[];
  hash: bigint;
  meta: Record<string, string>;
};

export type OperationArtifact = ArtifactBase & {
  kind: Extract<ArtifactKind, 'query' | 'mutation' | 'subscription'>;
  node: graphql.OperationDefinitionNode;
  variables: Variable[];
};

export type FragmentArtifact = ArtifactBase & {
  kind: Extract<ArtifactKind, 'fragment'>;
  on: DistributiveOmit<CompositeType, 'isNonNull' | 'isList'>;
  node: graphql.FragmentDefinitionNode;
};

export type Artifact = OperationArtifact | FragmentArtifact;

export type TypeBase = {
  name: string;
  isList: boolean;
  isNonNull: boolean;
};

export type TypenameType = {
  kind: '__typename';
  name: '__typename';
};

export type ScalarType = TypeBase & {
  kind: 'Scalar';
};

export type EnumType = TypeBase & {
  kind: 'Enum';
  values: string[];
};

export type ObjectType = TypeBase & {
  kind: 'Object';
};

export type InputObjectType = TypeBase & {
  kind: 'InputObject';
};

export type InterfaceType = TypeBase & {
  kind: 'Interface';
  implementations: string[];
};

export type UnionType = TypeBase & {
  kind: 'Union';
  members: string[];
};

export type CompositeType = ObjectType | InterfaceType | UnionType;

export type FieldSelectionBase = {
  name: string;
  alias?: string;
  arguments: Argument[];
  implicit?: boolean;
};

export type TypenameFieldSelection = FieldSelectionBase & {
  kind: 'TypenameField';
  name: '__typename';
  type: TypenameType;
};

export type ScalarFieldSelection = FieldSelectionBase & {
  kind: 'ScalarField';
  type: ScalarType;
};

export type EnumFieldSelection = FieldSelectionBase & {
  kind: 'EnumField';
  type: EnumType;
};

export type ObjectFieldSelection = FieldSelectionBase & {
  kind: 'ObjectField';
  type: CompositeType;
  children: Selection[];
};

export type FragmentSpreadSelection = {
  kind: 'FragmentSpread';
  type: CompositeType;
  name: string;
};

export type InlineFragmentSelection = {
  kind: 'InlineFragment';
  type: CompositeType;
  children: Selection[];
};

export type Selection =
  | TypenameFieldSelection
  | ScalarFieldSelection
  | EnumFieldSelection
  | ObjectFieldSelection
  | FragmentSpreadSelection
  | InlineFragmentSelection;

export type VariableValue = {
  kind: 'Variable';
  name: string;
};

export type ListValue = {
  kind: 'List';
  values: Value[];
};

export type ObjectValue = {
  kind: 'Object';
  fields: { name: string; value: Value }[];
};

export type ScalarValue = {
  kind: 'Scalar';
  value: unknown;
};

export type Value = VariableValue | ListValue | ObjectValue | ScalarValue;

export type Argument = {
  name: string;
  value: Value;
};

export type VariableBase = {
  name: string;
};

export type ScalarFieldVariable = VariableBase & {
  kind: 'ScalarField';
  type: ScalarType;
};

export type EnumFieldVariable = VariableBase & {
  kind: 'EnumField';
  type: EnumType;
};

export type InputObjectFieldVariable = VariableBase & {
  kind: 'InputObjectField';
  type: InputObjectType;
  children: Variable[];
};

export type Variable = ScalarFieldVariable | EnumFieldVariable | InputObjectFieldVariable;

export type Data = Record<string, unknown>;
export type Variables = Record<string, unknown> | undefined;

export type $ArtifactSchema<Kind extends ArtifactKind = ArtifactKind, Input extends Variables = Variables, Output extends Data = Data> = {
  $name: string;
  $kind: Kind;
  $input: Input;
  $output: Output;
  $meta: Record<string, unknown>;
};

export type ArtifactSchema<Schema extends $ArtifactSchema = $ArtifactSchema> = {
  kind: Schema['$kind'];
  name: Schema['$name'];
  source: string;
  selections: { operation: Selection[]; fragments: Record<string, Selection[]> };
  meta: Schema['$meta'];
};
