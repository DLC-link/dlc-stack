import { getEnv } from '../config/read-env-configs.js';
import fetch from 'cross-fetch';

export default class RouterWalletService {
    private static routerWallet: RouterWalletService;

    private constructor(private _address: string) {
        this._address = _address;
    }

    public static async getRouterWallet(): Promise<RouterWalletService> {
        if (!this.routerWallet) this.routerWallet = new RouterWalletService(getEnv('ROUTER_WALLET_ADDRESS'));
        return this.routerWallet;
    }

    public async getHealth(): Promise<Response | any> {
        try {
            const address = this._address + '/health';
            const res = await fetch(address);
            return await res.json();
        } catch (error) {
            console.error(error);
            return error;
        }
    }

    public async getInfo(): Promise<Response | any> {
        try {
            const address = this._address + '/info';
            const res = await fetch(address);
            return await res.json();
        } catch (error) {
            console.error(error);
            return error;
        }
    }

    public async getOffer(body: any): Promise<Response | any> {
        try {
            const address = this._address + '/offer';
            const res = await fetch(address, {
                method: 'POST',
                body: JSON.stringify(body),
                headers: { 'Content-Type': 'application/json' },
            });
            return await res.json();
        } catch (error) {
            console.error(error);
            return error;
        }
    }

    public async acceptOffer(body: any): Promise<Response | any> {
        try {
            const address = this._address + '/offer/accept';
            const res = await fetch(address, {
                method: 'PUT',
                body: JSON.stringify(body),
                headers: { 'Content-Type': 'application/json' },
            });
            return await res.json();
        } catch (error) {
            console.error(error);
            return error;
        }
    }
}
