export interface IOrder {
  id: string;
  title: string;
  price: number;
  active: boolean;
}

export class OrderService {
  // <ai:customize hint="Add API methods for Order">
  async getAll(): Promise<IOrder[]> {
    throw new Error("Not implemented");
  }

  async getById(id: string): Promise<IOrder> {
    throw new Error("Not implemented");
  }
  // </ai:customize>
}
