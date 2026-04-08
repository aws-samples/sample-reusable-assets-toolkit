import { Request, Response } from 'express';

export interface User {
  id: string;
  name: string;
}

@Injectable()
export class UserService {
  private users: Map<string, User> = new Map();

  async getUser(id: string): Promise<User | undefined> {
    return this.users.get(id);
  }

  async createUser(name: string): Promise<User> {
    const user: User = { id: crypto.randomUUID(), name };
    this.users.set(user.id, user);
    return user;
  }
}

export const handler = async (req: Request, res: Response) => {
  const service = new UserService();
  const user = await service.getUser(req.params.id);
  res.json(user);
};

export function formatUser(user: User): string {
  return `${user.id}: ${user.name}`;
}
