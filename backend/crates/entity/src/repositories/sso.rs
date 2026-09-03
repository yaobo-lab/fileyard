use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Statement};
use uuid::Uuid;

use crate::{entities::{sso_attribute_mappings, tenants, users}, DataResult};

pub struct NewSsoMapping {
    pub tenant_id: Uuid, pub protocol: String, pub provider_id: Uuid,
    pub attribute_name: String, pub attribute_value: String, pub match_type: String,
    pub target_role: String, pub target_custom_role_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>, pub priority: i32, pub enabled: bool,
}

pub struct SsoMappingPatch {
    pub attribute_name: String, pub attribute_value: String, pub match_type: String,
    pub target_role: String, pub target_custom_role_id: Option<Uuid>,
    pub target_department_id: Option<Uuid>, pub priority: i32, pub enabled: bool,
}

pub struct SsoRepository<'a> { db: &'a DatabaseConnection }

impl<'a> SsoRepository<'a> {
    pub(crate) fn new(db: &'a DatabaseConnection) -> Self { Self { db } }

    pub async fn mappings(&self, tenant_id: Uuid, protocol: &str, provider_id: Uuid) -> DataResult<Vec<sso_attribute_mappings::Model>> {
        Ok(sso_attribute_mappings::Entity::find()
            .filter(sso_attribute_mappings::Column::TenantId.eq(tenant_id))
            .filter(sso_attribute_mappings::Column::Protocol.eq(protocol))
            .filter(sso_attribute_mappings::Column::ProviderId.eq(provider_id))
            .order_by_desc(sso_attribute_mappings::Column::Priority)
            .order_by_asc(sso_attribute_mappings::Column::CreatedAt).all(self.db).await?)
    }

    pub async fn enabled_mappings(&self, protocol: &str, provider_id: Uuid) -> DataResult<Vec<sso_attribute_mappings::Model>> {
        Ok(sso_attribute_mappings::Entity::find().filter(sso_attribute_mappings::Column::Protocol.eq(protocol))
            .filter(sso_attribute_mappings::Column::ProviderId.eq(provider_id)).filter(sso_attribute_mappings::Column::Enabled.eq(true))
            .order_by_desc(sso_attribute_mappings::Column::Priority).order_by_asc(sso_attribute_mappings::Column::CreatedAt).all(self.db).await?)
    }

    fn stmt(&self, sql: &str, values: Vec<sea_orm::Value>) -> Statement { Statement::from_sql_and_values(self.db.get_database_backend(), sql, values) }

    pub async fn identity_user_id(&self, protocol: &str, provider_id: Uuid, subject: &str) -> DataResult<Option<Uuid>> {
        let sql = match protocol { "oidc" => "SELECT user_id FROM user_oidc_identities WHERE provider_id=$1 AND oidc_subject=$2", "saml" => "SELECT user_id FROM user_saml_identities WHERE provider_id=$1 AND saml_name_id=$2", _ => return Ok(None) };
        Ok(self.db.query_one(self.stmt(sql, vec![provider_id.into(),subject.into()])).await?.map(|r| r.try_get("","user_id")).transpose()?)
    }

    pub async fn active_user(&self, id: Uuid) -> DataResult<Option<users::Model>> { Ok(users::Entity::find_by_id(id).filter(users::Column::Status.eq("active")).one(self.db).await?) }
    pub async fn active_user_by_email(&self, tenant_id: Uuid, email: &str) -> DataResult<Option<users::Model>> { Ok(users::Entity::find().filter(users::Column::TenantId.eq(tenant_id)).filter(users::Column::Email.eq(email)).filter(users::Column::Status.eq("active")).one(self.db).await?) }
    pub async fn tenant(&self, id: Uuid) -> DataResult<Option<tenants::Model>> { Ok(tenants::Entity::find_by_id(id).one(self.db).await?) }

    pub async fn create_user(&self, tenant_id:Uuid,email:&str,name:&str,role:&str,custom_role_id:Option<Uuid>,provider:&str,department_id:Option<Uuid>)->DataResult<users::Model>{
        let now=chrono::Utc::now().into(); Ok(users::ActiveModel { id:Set(Uuid::new_v4()),tenant_id:Set(tenant_id),department_id:Set(department_id),custom_role_id:Set(custom_role_id),email:Set(email.into()),name:Set(name.into()),password_hash:Set(None),role:Set(role.into()),status:Set("active".into()),avatar_url:Set(None),allowed_tenant_ids:Set(None),allowed_department_ids:Set(None),totp_secret:Set(None),recovery_token:Set(None),recovery_token_expires_at:Set(None),password_changed_at:Set(None),suspended_at:Set(None),suspended_until:Set(None),suspension_reason:Set(None),dashboard_layout:Set(None),widget_config:Set(None),last_active_at:Set(None),created_at:Set(now),updated_at:Set(now),identity_provider:Set(provider.into()) }.insert(self.db).await?)
    }

    pub async fn set_hybrid(&self,id:Uuid)->DataResult<()>{let Some(user)=users::Entity::find_by_id(id).one(self.db).await? else{return Ok(())};let mut active:users::ActiveModel=user.into();active.identity_provider=Set("hybrid".into());active.updated_at=Set(chrono::Utc::now().into());active.update(self.db).await?;Ok(())}
    pub async fn touch_user(&self,id:Uuid)->DataResult<()>{let Some(user)=users::Entity::find_by_id(id).one(self.db).await? else{return Ok(())};let mut active:users::ActiveModel=user.into();active.last_active_at=Set(Some(chrono::Utc::now().into()));active.update(self.db).await?;Ok(())}

    pub async fn link_identity(&self,protocol:&str,user_id:Uuid,provider_id:Uuid,subject:&str,issuer:&str,email:Option<&str>,name:Option<&str>)->DataResult<()> {
        let (sql,values)=if protocol=="oidc"{("INSERT INTO user_oidc_identities (user_id,provider_id,oidc_subject,oidc_issuer,oidc_email,oidc_name) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (provider_id,oidc_subject) DO UPDATE SET oidc_email=EXCLUDED.oidc_email,oidc_name=EXCLUDED.oidc_name,updated_at=NOW()",vec![user_id.into(),provider_id.into(),subject.into(),issuer.into(),email.into(),name.into()])}else if protocol=="saml"{("INSERT INTO user_saml_identities (user_id,provider_id,saml_name_id,saml_email,saml_name) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (provider_id,saml_name_id) DO UPDATE SET saml_email=EXCLUDED.saml_email,saml_name=EXCLUDED.saml_name,updated_at=NOW()",vec![user_id.into(),provider_id.into(),subject.into(),email.into(),name.into()])}else{return Ok(())};
        self.db.execute(self.stmt(sql,values)).await?;Ok(())
    }

    pub async fn upsert_session(&self,user_id:Uuid,token_hash:&str,device:Option<&str>,ip:Option<&str>,fingerprint:&str)->DataResult<()> {
        self.db.execute(self.stmt("INSERT INTO user_sessions (user_id,token_hash,device_info,ip_address,fingerprint_hash,expires_at) VALUES ($1,$2,$3,$4::inet,$5,NOW()+INTERVAL '7 days') ON CONFLICT (user_id,fingerprint_hash) WHERE is_revoked=false AND fingerprint_hash IS NOT NULL DO UPDATE SET token_hash=EXCLUDED.token_hash,device_info=EXCLUDED.device_info,ip_address=EXCLUDED.ip_address,last_active_at=NOW(),expires_at=EXCLUDED.expires_at",vec![user_id.into(),token_hash.into(),device.into(),ip.into(),fingerprint.into()])).await?;Ok(())
    }

    pub async fn mapping(&self, tenant_id: Uuid, id: Uuid) -> DataResult<Option<sso_attribute_mappings::Model>> {
        Ok(sso_attribute_mappings::Entity::find_by_id(id).filter(sso_attribute_mappings::Column::TenantId.eq(tenant_id)).one(self.db).await?)
    }

    pub async fn create_mapping(&self, input: NewSsoMapping) -> DataResult<sso_attribute_mappings::Model> {
        let now = chrono::Utc::now().into();
        Ok(sso_attribute_mappings::ActiveModel {
            id: Set(Uuid::new_v4()), tenant_id: Set(input.tenant_id), protocol: Set(input.protocol), provider_id: Set(input.provider_id),
            attribute_name: Set(input.attribute_name), attribute_value: Set(input.attribute_value), match_type: Set(input.match_type),
            target_role: Set(input.target_role), target_custom_role_id: Set(input.target_custom_role_id), target_department_id: Set(input.target_department_id),
            priority: Set(input.priority), enabled: Set(input.enabled), created_at: Set(now), updated_at: Set(now),
        }.insert(self.db).await?)
    }

    pub async fn update_mapping(&self, existing: sso_attribute_mappings::Model, input: SsoMappingPatch) -> DataResult<sso_attribute_mappings::Model> {
        let mut model: sso_attribute_mappings::ActiveModel = existing.into();
        model.attribute_name = Set(input.attribute_name); model.attribute_value = Set(input.attribute_value);
        model.match_type = Set(input.match_type); model.target_role = Set(input.target_role);
        model.target_custom_role_id = Set(input.target_custom_role_id); model.target_department_id = Set(input.target_department_id);
        model.priority = Set(input.priority); model.enabled = Set(input.enabled); model.updated_at = Set(chrono::Utc::now().into());
        Ok(model.update(self.db).await?)
    }

    pub async fn delete_mapping(&self, tenant_id: Uuid, id: Uuid) -> DataResult<bool> {
        Ok(sso_attribute_mappings::Entity::delete_many().filter(sso_attribute_mappings::Column::Id.eq(id))
            .filter(sso_attribute_mappings::Column::TenantId.eq(tenant_id)).exec(self.db).await?.rows_affected > 0)
    }
}
