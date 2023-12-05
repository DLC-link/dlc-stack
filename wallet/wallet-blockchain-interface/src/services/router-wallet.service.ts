import fetch from 'cross-fetch';
import ConfigService from './config.service.js';

export default class RouterWalletService {
    private static routerWallet: RouterWalletService;

    private constructor(private _address: string) {
        this._address = _address;
    }

    public static async getRouterWallet(): Promise<RouterWalletService> {
        if (!this.routerWallet)
            this.routerWallet = new RouterWalletService(ConfigService.getSettings()['router-wallet-address']);
        return this.routerWallet;
    }

    public async getHealth(): Promise<Response | any> {
        try {
            const address = this._address + '/health';
            const res = await fetch(address);
            return res;
        } catch (error) {
            console.error(error);
            return error;
        }
    }

    public async getInfo(): Promise<Response | any> {
        try {
            const address = this._address + '/info';
            const res = await fetch(address);
            return res;
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
            return res;
        } catch (error) {
            console.error(error);
            throw error;
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
            return res;
        } catch (error) {
            console.error(error);
            return error;
        }
    }

    public async getChainForUUID(uuid: string): Promise<Response | any> {
        try {
            const address = this._address + `/get_chain/${uuid}`;
            const res = await fetch(address);
            return res;
        } catch (error) {
            console.error(error);
            return error;
        }
    }
}
