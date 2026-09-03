use crate::{
    entities::{saml_auth_states, tenant_saml_providers, user_saml_identities, users},
    DataResult,
};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection,
    EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Statement,
};
use uuid::Uuid;

pub struct NewSamlProvider {
    pub tenant_id: Uuid,
    pub name: String,
    pub slug: String,
    pub provider_type: String,
    pub idp_entity_id: String,
    pub idp_sso_url: String,
    pub idp_slo_url: Option<String>,
    pub idp_metadata_url: Option<String>,
    pub idp_signing_certificate: String,
    pub sp_entity_id: String,
    pub nameid_format: String,
    pub sso_binding: String,
    pub attribute_email: String,
    pub attribute_name: String,
    pub auto_provision: bool,
    pub default_role: String,
    pub default_custom_role_id: Option<Uuid>,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Vec<String>,
    pub trust_idp_mfa: bool,
    pub enabled: bool,
}
pub struct SamlProviderPatch {
    pub name: Option<String>,
    pub slug: Option<String>,
    pub provider_type: Option<String>,
    pub idp_entity_id: Option<String>,
    pub idp_sso_url: Option<String>,
    pub idp_slo_url: Option<String>,
    pub idp_metadata_url: Option<String>,
    pub idp_signing_certificate: Option<String>,
    pub nameid_format: Option<String>,
    pub sso_binding: Option<String>,
    pub attribute_email: Option<String>,
    pub attribute_name: Option<String>,
    pub auto_provision: Option<bool>,
    pub default_role: Option<String>,
    pub default_custom_role_id: Option<Uuid>,
    pub default_department_id: Option<Uuid>,
    pub email_domains: Option<Vec<String>>,
    pub trust_idp_mfa: Option<bool>,
    pub enabled: Option<bool>,
}
pub struct SamlRepository<'a> {
    db: &'a DatabaseConnection,
}
impl<'a> SamlRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self {
        Self { db }
    }
    fn stmt(&self, s: &str, v: Vec<sea_orm::Value>) -> Statement {
        Statement::from_sql_and_values(self.db.get_database_backend(), s, v)
    }
    pub async fn provider(&self, id: Uuid) -> DataResult<Option<tenant_saml_providers::Model>> {
        Ok(tenant_saml_providers::Entity::find_by_id(id)
            .one(self.db)
            .await?)
    }
    pub async fn enabled_provider(
        &self,
        id: Uuid,
    ) -> DataResult<Option<tenant_saml_providers::Model>> {
        Ok(tenant_saml_providers::Entity::find_by_id(id)
            .filter(tenant_saml_providers::Column::Enabled.eq(true))
            .one(self.db)
            .await?)
    }
    pub async fn tenant_provider(
        &self,
        t: Uuid,
        id: Uuid,
        enabled: Option<bool>,
    ) -> DataResult<Option<tenant_saml_providers::Model>> {
        let mut q = tenant_saml_providers::Entity::find_by_id(id)
            .filter(tenant_saml_providers::Column::TenantId.eq(t));
        if let Some(v) = enabled {
            q = q.filter(tenant_saml_providers::Column::Enabled.eq(v))
        }
        Ok(q.one(self.db).await?)
    }
    pub async fn create_state(
        &self,
        relay: String,
        request: String,
        provider: Uuid,
        tenant: Uuid,
        user: Option<Uuid>,
    ) -> DataResult<()> {
        let now = chrono::Utc::now();
        saml_auth_states::ActiveModel {
            id: Set(Uuid::new_v4()),
            relay_state: Set(relay),
            authn_request_id: Set(request),
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
    pub async fn consume_state(&self, relay: &str) -> DataResult<Option<saml_auth_states::Model>> {
        let row = saml_auth_states::Entity::find()
            .filter(saml_auth_states::Column::RelayState.eq(relay))
            .filter(saml_auth_states::Column::ExpiresAt.gt(chrono::Utc::now().fixed_offset()))
            .one(self.db)
            .await?;
        if row.is_some() {
            saml_auth_states::Entity::delete_many()
                .filter(saml_auth_states::Column::RelayState.eq(relay))
                .exec(self.db)
                .await?;
        }
        Ok(row)
    }
    pub async fn consume_assertion(
        &self,
        id: &str,
        provider: Uuid,
        expiry: chrono::DateTime<chrono::Utc>,
    ) -> DataResult<bool> {
        let r=self.db.execute(self.stmt("INSERT INTO saml_consumed_assertions (assertion_id,provider_id,consumed_at,not_on_or_after) VALUES ($1,$2,NOW(),$3) ON CONFLICT (assertion_id) DO NOTHING",vec![id.into(),provider.into(),expiry.into()])).await?;
        Ok(r.rows_affected() > 0)
    }
    pub async fn list(&self, t: Uuid) -> DataResult<Vec<tenant_saml_providers::Model>> {
        Ok(tenant_saml_providers::Entity::find()
            .filter(tenant_saml_providers::Column::TenantId.eq(t))
            .order_by_desc(tenant_saml_providers::Column::CreatedAt)
            .all(self.db)
            .await?)
    }
    pub async fn create(&self, n: NewSamlProvider) -> DataResult<tenant_saml_providers::Model> {
        let now = chrono::Utc::now().into();
        let p = tenant_saml_providers::ActiveModel {
            id: Set(Uuid::new_v4()),
            tenant_id: Set(n.tenant_id),
            name: Set(n.name),
            slug: Set(n.slug),
            provider_type: Set(n.provider_type),
            idp_entity_id: Set(n.idp_entity_id),
            idp_sso_url: Set(n.idp_sso_url),
            idp_slo_url: Set(n.idp_slo_url),
            idp_metadata_url: Set(n.idp_metadata_url),
            idp_metadata_xml: Set(None),
            idp_signing_certificate: Set(n.idp_signing_certificate),
            sp_entity_id: Set(n.sp_entity_id),
            nameid_format: Set(n.nameid_format),
            request_signing: Set(false),
            want_assertions_signed: Set(true),
            want_response_signed: Set(true),
            sp_signing_key_encrypted: Set(None),
            sp_signing_cert: Set(None),
            sso_binding: Set(n.sso_binding),
            attribute_email: Set(n.attribute_email),
            attribute_name: Set(n.attribute_name),
            auto_provision: Set(n.auto_provision),
            default_role: Set(n.default_role),
            default_custom_role_id: Set(n.default_custom_role_id),
            default_department_id: Set(n.default_department_id),
            email_domains: Set(n.email_domains),
            trust_idp_mfa: Set(n.trust_idp_mfa),
            enabled: Set(n.enabled),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(self.db)
        .await?;
        self.auth(n.tenant_id, true).await?;
        Ok(p)
    }
    async fn auth(&self, t: Uuid, add: bool) -> DataResult<()> {
        let sql = if add {
            "UPDATE tenants SET auth_methods=CASE WHEN NOT ('saml'=ANY(auth_methods)) THEN array_append(auth_methods,'saml') ELSE auth_methods END,updated_at=NOW() WHERE id=$1"
        } else {
            "UPDATE tenants SET auth_methods=array_remove(auth_methods,'saml'),updated_at=NOW() WHERE id=$1"
        };
        self.db.execute(self.stmt(sql, vec![t.into()])).await?;
        Ok(())
    }
    pub async fn update(
        &self,
        t: Uuid,
        id: Uuid,
        p: SamlProviderPatch,
    ) -> DataResult<Option<tenant_saml_providers::Model>> {
        let Some(r) = self.tenant_provider(t, id, None).await? else {
            return Ok(None);
        };
        let mut a: tenant_saml_providers::ActiveModel = r.into();
        macro_rules! s {
            ($f:ident) => {
                if let Some(v) = p.$f {
                    a.$f = Set(v)
                }
            };
        }
        s!(name);
        s!(slug);
        s!(provider_type);
        s!(idp_entity_id);
        s!(idp_sso_url);
        if let Some(v) = p.idp_slo_url {
            a.idp_slo_url = Set(Some(v))
        }
        if let Some(v) = p.idp_metadata_url {
            a.idp_metadata_url = Set(Some(v))
        }
        s!(idp_signing_certificate);
        s!(nameid_format);
        s!(sso_binding);
        s!(attribute_email);
        s!(attribute_name);
        s!(auto_provision);
        s!(default_role);
        if let Some(v) = p.default_custom_role_id {
            a.default_custom_role_id = Set(Some(v))
        }
        if let Some(v) = p.default_department_id {
            a.default_department_id = Set(Some(v))
        }
        s!(email_domains);
        s!(trust_idp_mfa);
        s!(enabled);
        a.updated_at = Set(chrono::Utc::now().into());
        Ok(Some(a.update(self.db).await?))
    }
    pub async fn saml_only_count(&self, t: Uuid, id: Uuid) -> DataResult<i64> {
        let r=self.db.query_one(self.stmt("SELECT COUNT(*) count FROM users u JOIN user_saml_identities i ON i.user_id=u.id WHERE i.provider_id=$1 AND u.identity_provider='saml' AND u.tenant_id=$2",vec![id.into(),t.into()])).await?.unwrap();
        Ok(r.try_get("", "count")?)
    }
    pub async fn delete(&self, t: Uuid, id: Uuid) -> DataResult<bool> {
        let r = tenant_saml_providers::Entity::delete_many()
            .filter(tenant_saml_providers::Column::Id.eq(id))
            .filter(tenant_saml_providers::Column::TenantId.eq(t))
            .exec(self.db)
            .await?;
        if tenant_saml_providers::Entity::find()
            .filter(tenant_saml_providers::Column::TenantId.eq(t))
            .count(self.db)
            .await?
            == 0
        {
            self.auth(t, false).await?
        }
        Ok(r.rows_affected > 0)
    }
    pub async fn link_identity(
        &self,
        user: Uuid,
        provider: Uuid,
        subject: &str,
        format: Option<&str>,
        session: Option<&str>,
        email: Option<&str>,
        name: Option<&str>,
    ) -> DataResult<()> {
        self.db.execute(self.stmt("INSERT INTO user_saml_identities(user_id,provider_id,saml_name_id,saml_name_id_format,saml_session_index,saml_email,saml_name) VALUES($1,$2,$3,$4,$5,$6,$7) ON CONFLICT(provider_id,saml_name_id) DO UPDATE SET saml_email=EXCLUDED.saml_email,saml_name=EXCLUDED.saml_name,saml_session_index=EXCLUDED.saml_session_index,updated_at=NOW()",vec![user.into(),provider.into(),subject.into(),format.into(),session.into(),email.into(),name.into()])).await?;
        Ok(())
    }
    pub async fn touch_identity(
        &self,
        provider: Uuid,
        subject: &str,
        email: Option<&str>,
        name: Option<&str>,
        session: Option<&str>,
    ) -> DataResult<()> {
        self.db.execute(self.stmt("UPDATE user_saml_identities SET last_login_at=NOW(),login_count=login_count+1,saml_email=$3,saml_name=$4,saml_session_index=$5,updated_at=NOW() WHERE provider_id=$1 AND saml_name_id=$2",vec![provider.into(),subject.into(),email.into(),name.into(),session.into()])).await?;
        Ok(())
    }
    pub async fn identity(
        &self,
        user: Uuid,
        id: Uuid,
    ) -> DataResult<Option<user_saml_identities::Model>> {
        Ok(user_saml_identities::Entity::find_by_id(id)
            .filter(user_saml_identities::Column::UserId.eq(user))
            .one(self.db)
            .await?)
    }
    pub async fn identities(&self, user: Uuid) -> DataResult<Vec<user_saml_identities::Model>> {
        Ok(user_saml_identities::Entity::find()
            .filter(user_saml_identities::Column::UserId.eq(user))
            .order_by_desc(user_saml_identities::Column::CreatedAt)
            .all(self.db)
            .await?)
    }
    pub async fn counts(&self, user: Uuid) -> DataResult<(u64, u64)> {
        let s = user_saml_identities::Entity::find()
            .filter(user_saml_identities::Column::UserId.eq(user))
            .count(self.db)
            .await?;
        let o = crate::entities::user_oidc_identities::Entity::find()
            .filter(crate::entities::user_oidc_identities::Column::UserId.eq(user))
            .count(self.db)
            .await?;
        Ok((s, o))
    }
    pub async fn delete_identity(&self, user: Uuid, id: Uuid) -> DataResult<bool> {
        Ok(user_saml_identities::Entity::delete_many()
            .filter(user_saml_identities::Column::Id.eq(id))
            .filter(user_saml_identities::Column::UserId.eq(user))
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
