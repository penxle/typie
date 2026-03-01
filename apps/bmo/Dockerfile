FROM public.ecr.aws/lambda/nodejs:24-arm64

ENV HOME=/tmp

RUN dnf install -y tar zstd && dnf clean all
RUN npm install -g @anthropic-ai/claude-code

WORKDIR ${LAMBDA_TASK_ROOT}
COPY package.json ./
RUN npm install --omit=dev

COPY src/ ./src/
RUN echo 'export { handler } from "./src/worker.ts";' > handler.js

CMD ["handler.handler"]
