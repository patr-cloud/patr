import { FindAccount } from "oidc-provider";
import { getUserByUserid } from "../../../models/database-modules/user";

export default class Account {
    static findAccount: FindAccount = async (ctx, id) => {
        const account = await getUserByUserid(id);
        if (!account) {
            return undefined;
        }

        return {
            accountId: id,
            async claims() {
                return {
                    sub: id,
                };
            },
        };
    }
}
