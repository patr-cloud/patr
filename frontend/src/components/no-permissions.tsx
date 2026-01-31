import PageContainerBody from "./page/body";
import PageContainer from "./page/container";
import PageContainerHead from "./page/head";

const NoPermissionPage = (props: { title: string; subTitle?: string; titleUrl?: string; message: string }) => {
	return (
		<PageContainer>
			<PageContainerHead titleUrl={props.titleUrl} title={props.title} subTitle={props.subTitle} />
			<PageContainerBody>
				<div class="p-6 bg-red-100 text-error rounded">{props.message}</div>
			</PageContainerBody>
		</PageContainer>
	);
};

export default NoPermissionPage;
