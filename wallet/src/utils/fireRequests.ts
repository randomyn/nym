import { Console } from 'src/utils/console';

export type TauriReq<Req extends (...args: any[]) => Promise<any>> = {
  name: Req extends { name: infer N } ? N : never;
  request: () => ReturnType<Req>;
  onFulfilled: (value: Awaited<ReturnType<Req>>) => void;
};

async function fireRequests(requests: TauriReq<any>[]) {
  const promises = await Promise.allSettled(requests.map((r) => r.request()));

  promises.forEach((res, index) => {
    if (res.status === 'rejected') {
      Console.warn(`${requests[index].name} request fails`, res.reason);
    }
    if (res.status === 'fulfilled') {
      requests[index].onFulfilled(res.value as any);
    }
  });
}

export default fireRequests;
