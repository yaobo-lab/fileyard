use crate::{
    entities::{oidc_oauth_states, tenant_oidc_providers, tenants, user_oidc_identities, users},
    DataResult,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;
pub struct NewOidcProvider {
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub provider_type: String,
    pub issuer_url: String,
    pub client_id: String,
    pub client_secret: String,
    pub scopes: String,
    pub auto_provision: bool,
    pub default_role: String,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Vec<String>,
    pub trust_idp_mfa: bool,
    pub enabled: bool,
}
pub struct OidcProviderPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub provider_type: Option<String>,
    pub issuer_url: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub scopes: Option<String>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Option<Vec<String>>,
    pub trust_idp_mfa: Option<bool>,
    pub enabled: Option<bool>,
}
pub struct OidcRepository<'a> {
    db: &'a DatabaseConnection,
}
impl<'a> OidcRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, s: &str, v: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), s, v)
    }
    pub async fn discover(
        &self,
        domain: &str,
    ) -> DataResult<Vec<(Uuid, String, String, String, String)>> {
        let mut out = Vec::new();
        let rows=self.db.query_all(self.stmt("SELECT p.id,p.name,p.slug,p.provider_type FROM tenant_oidc_providers p WHERE p.enabled=true AND $1=ANY(p.email_domains)",vec![domain.into()])).await?;
        for r in rows {
            out.push((
                r.try_get("", "id")?,
                r.try_get("", "name")?,
                r.try_get("", "slug")?,
                r.try_get("", "provider_type")?,
                "oidc".into(),
            ))
        }
        let rows=self.db.query_all(self.stmt("SELECT p.id,p.name,p.slug,p.provider_type FROM tenant_saml_providers p WHERE p.enabled=true AND $1=ANY(p.email_domains)",vec![domain.into()])).await?;
        for r in rows {
            out.push((
                r.try_get("", "id")?,
                r.try_get("", "name")?,
                r.try_get("", "slug")?,
                r.try_get("", "provider_type")?,
                "saml".into(),
            ))
        }
        Ok(out)
    }
    pub async fn provider(&self, id: Uuid) -> DataResult<Option<tenant_oidc_providers::Model>> {
        Ok(tenant_oidc_providers::Entity::find_by_id(id)
            .one(self.db)
            .await?)
    }
    pub async fn tenant_provider(
        &self,
        t: Uuid,
        id: Uuid,
        enabled: Option<bool>,
    ) -> DataResult<Option<tenant_oidc_providers::Model>> {
        let mut q = tenant_oidc_providers::Entity::find_by_id(id)
            .filter(tenant_oidc_providers::Column::TenantId.eq(t));
        if let Some(v) = enabled {
            q = q.filter(tenant_oidc_providers::Column::Enabled.eq(v))
        }
        Ok(q.one(self.db).await?)
    }
    pub async fn enabled_provider(
        &self,
        id: Uuid,
    ) -> DataResult<Option<tenant_oidc_providers::Model>> {
        Ok(tenant_oidc_providers::Entity::find_by_id(id)
            .filter(tenant_oidc_providers::Column::Enabled.eq(true))
            .one(self.db)
            .await?)
    }
    pub async fn tenant_sso_only_for_provider(&self, id: Uuid) -> DataResult<bool> {
        let t=self.db.query_one(self.stmt("SELECT t.auth_methods FROM tenants t WHERE t.id=(SELECT tenant_id FROM tenant_oidc_providers WHERE id=$1 UNION ALL SELECT tenant_id FROM tenant_saml_providers WHERE id=$1 LIMIT 1)",vec![id.into()])).await?;
        Ok(t.map(|r| r.try_get::<Vec<String>>("", "auth_methods"))
            .transpose()?
            .map(|m| !m.contains(&"local".to_string()))
            .unwrap_or(false))
    }
    pub async fn create_state(
        &self,
        state: String,
        nonce: String,
        provider: Uuid,
        tenant: Uuid,
        user: Option<Uuid>,
    ) -> DataResult<()> {
        let now = chrono::Utc::now();
        oidc_oauth_states::ActiveModel {
            state: Set(state),
            nonce: Set(nonce),
            provider_id: Set(provider),
            tenant_id: Set(tenant),
            user_id: Set(user),
            created_at: Set(now.into()),
            expires_at: Set((now + chrono::Duration::minutes(10)).into()),
        }
        .insert(self.db)
        .await?;
        Ok(())
    }
    pub async fn consume_state(&self, state: &str) -> DataResult<Option<oidc_oauth_states::Model>> {
        let row = oidc_oauth_states::Entity::find_by_id(state)
            .filter(oidc_oauth_states::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .one(self.db)
            .await?;
        if row.is_some() {
            oidc_oauth_states::Entity::delete_by_id(state)
                .exec(self.db)
                .await?;
        }
        Ok(row)
    }
    pub async fn list(&self, t: Uuid) -> DataResult<Vec<tenant_oidc_providers::Model>> {
        Ok(tenant_oidc_providers::Entity::find()
            .filter(tenant_oidc_providers::Column::TenantId.eq(t))
            .order_by_desc(tenant_oidc_providers::Column::CreatedAt)
            .all(self.db)
            .await?)
    }
    pub async fn create(&self, n: NewOidcProvider) -> DataResult<tenant_oidc_providers::Model> {
        let now = chrono::Utc::now().into();
        let p = tenant_oidc_providers::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(n.tenant_id),
            name: Set(n.name),
            slug: Set(n.slug),
            provider_type: Set(n.provider_type),
            issuer_url: Set(n.issuer_url),
            client_id: Set(n.client_id),
            client_secret_encrypted: Set(n.client_secret),
            scopes: Set(n.scopes),
            authorization_endpoint: Set(None),
            token_endpoint: Set(None),
            userinfo_endpoint: Set(None),
            jwks_uri: Set(None),
            auto_provision: Set(n.auto_provision),
            default_role: Set(n.default_role),
            default_department_id: Set(n.default_department_id),
            email_domains: Set(n.email_domains),
            trust_idp_mfa: Set(n.trust_idp_mfa),
            enabled: Set(n.enabled),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db)
        .await?;
        self.add_auth_method(n.tenant_id, "oidc").await?;
        Ok(p)
    }
    pub async fn update(
        &self,
        t: Uuid,
        id: Uuid,
        p: OidcProviderPatch,
    ) -> DataResult<Option<tenant_oidc_providers::Model>> {
        let Some(r) = self.tenant_provider(t, id, None).await? else {
            return Ok(None);
        };
        let mut a: tenant_oidc_providers::ActiveModel = r.into();
        macro_rules! setv {
            ($f:ident) => {
                if let Some(v) = p.$f {
                    a.$f = Set(v)
                }
            };
        }
        setv!(name);
        setv!(slug);
        setv!(provider_type);
        setv!(issuer_url);
        setv!(client_id);
        if let Some(v) = p.client_secret {
            a.client_secret_encrypted = Set(v)
        }
        setv!(scopes);
        setv!(auto_provision);
        setv!(default_role);
        if let Some(v) = p.default_department_id {
            a.default_department_id = Set(Some(v))
        }
        setv!(email_domains);
        setv!(trust_idp_mfa);
        setv!(enabled);
        a.updated_at = Set(chrono::Utc::now().into());
        Ok(Some(a.update(self.db).await?))
    }
    async fn add_auth_method(&self, t: Uuid, m: &str) -> DataResult<()> {
        let Some(r) = tenants::Entity::find_by_id(t).one(self.db).await? else {
            return Ok(());
        };
        let mut a: tenants::ActiveModel = r.into();
        let mut methods = a.auth_methods.take().unwrap_or_default();
        if !methods.iter().any(|x| x == m) {
            methods.push(m.into())
        }
        a.auth_methods = Set(methods);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    async fn remove_auth_method(&self, t: Uuid, m: &str) -> DataResult<()> {
        let Some(r) = tenants::Entity::find_by_id(t).one(self.db).await? else {
            return Ok(());
        };
        let mut a: tenants::ActiveModel = r.into();
        let mut methods = a.auth_methods.take().unwrap_or_default();
        methods.retain(|x| x != m);
        a.auth_methods = Set(methods);
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
    pub async fn oidc_only_count(&self, t: Uuid, id: Uuid) -> DataResult<i64> {
        let r=self.db.query_one(self.stmt("SELECT COUNT(*) AS count FROM users u JOIN user_oidc_identities i ON i.user_id=u.id WHERE i.provider_id=$1 AND u.identity_provider='oidc' AND u.tenant_id=$2",vec![id.into(),t.into()])).await?.unwrap();
        Ok(r.try_get("", "count")?)
    }
    pub async fn delete(&self, t: Uuid, id: Uuid) -> DataResult<bool> {
        let r = tenant_oidc_providers::Entity::delete_many()
            .filter(tenant_oidc_providers::Column::Id.eq(id))
            .filter(tenant_oidc_providers::Column::TenantId.eq(t))
            .exec(self.db)
            .await?;
        if tenant_oidc_providers::Entity::find()
            .filter(tenant_oidc_providers::Column::TenantId.eq(t))
            .count(self.db)
            .await?
            == 0
        {
            self.remove_auth_method(t, "oidc").await?
        }
        Ok(r.rows_affected > 0)
    }
    pub async fn touch_identity(
        &self,
        provider: Uuid,
        subject: &str,
        email: Option<String>,
        name: Option<String>,
    ) -> DataResult<()> {
        if let Some(r) = user_oidc_identities::Entity::find()
            .filter(user_oidc_identities::Column::ProviderId.eq(provider))
            .filter(user_oidc_identities::Column::OidcSubject.eq(subject))
            .one(self.db)
            .await?
        {
            let mut a: user_oidc_identities::ActiveModel = r.into();
            a.last_login_at = Set(Some(chrono::Utc::now().into()));
            a.login_count = Set(a.login_count.take().unwrap_or_default() + 1);
            a.oidc_email = Set(email);
            a.oidc_name = Set(name);
            a.updated_at = Set(chrono::Utc::now().into());
            a.update(self.db).await?;
        }
        Ok(())
    }
    pub async fn identity(
        &self,
        user: Uuid,
        id: Uuid,
    ) -> DataResult<Option<user_oidc_identities::Model>> {
        Ok(user_oidc_identities::Entity::find_by_id(id)
            .filter(user_oidc_identities::Column::UserId.eq(user))
            .one(self.db)
            .await?)
    }
    pub async fn identities(&self, user: Uuid) -> DataResult<Vec<user_oidc_identities::Model>> {
        Ok(user_oidc_identities::Entity::find()
            .filter(user_oidc_identities::Column::UserId.eq(user))
            .order_by_desc(user_oidc_identities::Column::CreatedAt)
            .all(self.db)
            .await?)
    }
    pub async fn identity_count(&self, user: Uuid) -> DataResult<u64> {
        Ok(user_oidc_identities::Entity::find()
            .filter(user_oidc_identities::Column::UserId.eq(user))
            .count(self.db)
            .await?)
    }
    pub async fn delete_identity(&self, user: Uuid, id: Uuid) -> DataResult<bool> {
        Ok(user_oidc_identities::Entity::delete_many()
            .filter(user_oidc_identities::Column::Id.eq(id))
            .filter(user_oidc_identities::Column::UserId.eq(user))
            .exec(self.db)
            .await?
            .rows_affected
            > 0)
    }
    pub async fn set_local(&self, user: Uuid) -> DataResult<()> {
        let Some(r) = users::Entity::find_by_id(user).one(self.db).await? else {
            return Ok(());
        };
        let mut a: users::ActiveModel = r.into();
        a.identity_provider = Set("local".into());
        a.updated_at = Set(chrono::Utc::now().into());
        a.update(self.db).await?;
        Ok(())
    }
}
