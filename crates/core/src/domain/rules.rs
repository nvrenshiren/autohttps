//! 状态机规则(证书 10 态的可用操作判定)—— core 服务层强制,api 据此门控(设计 H4)。
//! 转移规则权威:flows/certificates §2.3(引用不复述)。

use crate::domain::enums::CertificateStatus::{self, *};

impl CertificateStatus {
    /// 进行中态(签发中/续签中/吊销中)—— 不可删除、不可直接动作,取消经 tasks。
    pub fn is_in_progress(self) -> bool {
        matches!(self, Issuing | Renewing | Revoking)
    }

    /// 本地是否已有证书文件 → 可导出。非文件态:pending_issue / issuing / issue_failed(§2.8)。
    pub fn is_exportable(self) -> bool {
        !matches!(self, PendingIssue | Issuing | IssueFailed)
    }

    /// 可删除:非进行中态(§2.7)。
    pub fn can_delete(self) -> bool {
        !self.is_in_progress()
    }

    /// dashboard "失败数" 口径:issue_failed + renewal_failed + expired(dashboard §2)。
    pub fn is_failed(self) -> bool {
        matches!(self, IssueFailed | RenewalFailed | Expired)
    }

    /// 可续签/再获取:valid(T7)/ expiring_soon(T9)/ expired(T17)/ revoked(T20)(§2.4)。
    pub fn can_renew(self) -> bool {
        matches!(self, Valid | ExpiringSoon | Expired | Revoked)
    }

    /// 可失败重试:issue_failed(T5)/ renewal_failed(T14)(§2.5)。
    pub fn can_retry(self) -> bool {
        matches!(self, IssueFailed | RenewalFailed)
    }

    /// 可吊销:valid(T8)/ expiring_soon(T11)/ renewal_failed(T16)(§2.6,权威转移表不含 expired)。
    pub fn can_revoke(self) -> bool {
        matches!(self, Valid | ExpiringSoon | RenewalFailed)
    }

    /// dashboard 待处理触发集(§3.1):expired / issue_failed / renewal_failed / expiring_soon。
    pub fn is_pending_attention(self) -> bool {
        matches!(self, Expired | IssueFailed | RenewalFailed | ExpiringSoon)
    }

    /// 待处理清单排序权重(越小越靠前):已过期居首 → 其余失败 → 即将到期(§3.4)。
    pub fn pending_sort_rank(self) -> u8 {
        match self {
            Expired => 0,
            IssueFailed | RenewalFailed => 1,
            ExpiringSoon => 2,
            _ => 3,
        }
    }

    /// 域名"证书态投影"紧急度(越小越紧急)—— 失败 > 即将到期 > 进行中/待签 > 有效 > 已吊销(设计 §3.3)。
    pub fn projection_urgency(self) -> u8 {
        match self {
            Expired => 0,
            IssueFailed | RenewalFailed => 1,
            ExpiringSoon => 2,
            PendingIssue | Issuing | Renewing | Revoking => 3,
            Valid => 4,
            Revoked => 5,
        }
    }
}

/// 从一组关联证书态中取"最紧急"投影(域名列表/详情 worstCertificateStatus,DS3)。
pub fn worst_projection(statuses: &[CertificateStatus]) -> Option<CertificateStatus> {
    statuses
        .iter()
        .copied()
        .min_by_key(|s| s.projection_urgency())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_progress_states() {
        for s in [Issuing, Renewing, Revoking] {
            assert!(s.is_in_progress(), "{s:?} 应为进行中");
            assert!(!s.can_delete(), "{s:?} 进行中不可删除");
        }
        for s in [Valid, Expired, Revoked, IssueFailed, PendingIssue] {
            assert!(!s.is_in_progress(), "{s:?} 非进行中");
        }
    }

    #[test]
    fn exportable_states() {
        // 非文件态:pending_issue / issuing / issue_failed(§2.8)
        assert!(!PendingIssue.is_exportable());
        assert!(!Issuing.is_exportable());
        assert!(!IssueFailed.is_exportable());
        for s in [
            Valid,
            ExpiringSoon,
            Renewing,
            RenewalFailed,
            Expired,
            Revoking,
            Revoked,
        ] {
            assert!(s.is_exportable(), "{s:?} 应可导出");
        }
    }

    #[test]
    fn renewable_states() {
        // valid / expiring_soon / expired / revoked(§2.4)
        for s in [Valid, ExpiringSoon, Expired, Revoked] {
            assert!(s.can_renew(), "{s:?} 应可续签");
        }
        for s in [
            PendingIssue,
            Issuing,
            IssueFailed,
            Renewing,
            RenewalFailed,
            Revoking,
        ] {
            assert!(!s.can_renew(), "{s:?} 不可续签");
        }
    }

    #[test]
    fn retryable_states() {
        // issue_failed / renewal_failed(§2.5)
        assert!(IssueFailed.can_retry());
        assert!(RenewalFailed.can_retry());
        for s in [
            PendingIssue,
            Issuing,
            Valid,
            ExpiringSoon,
            Renewing,
            Expired,
            Revoking,
            Revoked,
        ] {
            assert!(!s.can_retry(), "{s:?} 不可重试");
        }
    }

    #[test]
    fn revocable_states() {
        // valid / expiring_soon / renewal_failed(§2.6,权威转移表不含 expired)
        for s in [Valid, ExpiringSoon, RenewalFailed] {
            assert!(s.can_revoke(), "{s:?} 应可吊销");
        }
        assert!(!Expired.can_revoke(), "expired 不可吊销(权威转移表)");
        for s in [
            PendingIssue,
            Issuing,
            IssueFailed,
            Renewing,
            Revoking,
            Revoked,
        ] {
            assert!(!s.can_revoke(), "{s:?} 不可吊销");
        }
    }

    #[test]
    fn failed_and_pending_sets() {
        // 失败口径:issue_failed + renewal_failed + expired(dashboard §2)
        for s in [IssueFailed, RenewalFailed, Expired] {
            assert!(s.is_failed(), "{s:?} 应计入失败");
        }
        // 待处理触发集:expired / issue_failed / renewal_failed / expiring_soon(§3.1)
        for s in [Expired, IssueFailed, RenewalFailed, ExpiringSoon] {
            assert!(s.is_pending_attention(), "{s:?} 应入待处理集");
        }
        for s in [PendingIssue, Issuing, Valid, Renewing, Revoking, Revoked] {
            assert!(!s.is_pending_attention(), "{s:?} 不入待处理集");
        }
    }

    #[test]
    fn pending_sort_order() {
        // expired 居首 → 其余失败 → 即将到期(§3.4)
        assert!(Expired.pending_sort_rank() < IssueFailed.pending_sort_rank());
        assert!(RenewalFailed.pending_sort_rank() < ExpiringSoon.pending_sort_rank());
    }

    #[test]
    fn worst_projection_picks_most_urgent() {
        use super::worst_projection;
        assert_eq!(worst_projection(&[]), None);
        assert_eq!(worst_projection(&[Valid, ExpiringSoon]), Some(ExpiringSoon));
        // 失败比即将到期更紧急,与切片顺序无关
        assert_eq!(
            worst_projection(&[Valid, ExpiringSoon, RenewalFailed, Revoked]),
            Some(RenewalFailed)
        );
        assert_eq!(worst_projection(&[Issuing, Valid]), Some(Issuing));
    }
}
