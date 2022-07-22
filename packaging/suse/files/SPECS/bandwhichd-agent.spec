Name:          bandwhichd-agent
Version:       0.37.0
Release:       1
License:       MIT
Group:         System/Monitoring
Summary:       bandwhichd agent publishing measurements
URL:           https://github.com/neuland/bandwhichd-agent
BuildRequires: systemd-rpm-macros
BuildRoot:     %{_buildrootdir}

%description
Publish current information about the host, its network interfaces, sockets and
network utilization to the bandwhichd monitoring system

%prep

%build

%install
rm -rf $RPM_BUILD_ROOT
mkdir -p $RPM_BUILD_ROOT%{_sbindir} $RPM_BUILD_ROOT%{_sysconfdir}/%{name} $RPM_BUILD_ROOT%{_unitdir}
cp %{name} $RPM_BUILD_ROOT%{_sbindir}/%{name}
cp %{name}.env $RPM_BUILD_ROOT%{_sysconfdir}/%{name}/%{name}.env
cp %{name}.service $RPM_BUILD_ROOT%{_unitdir}/%{name}.service

%pre
%systemd_pre %{name}.service

%post
%systemd_post %{name}.service

%preun
%systemd_preun %{name}.service

%postun
%systemd_postun_with_restart %{name}.service

%clean
rm -rf $RPM_BUILD_ROOT

%files
%defattr(644,root,root,755)
%attr(755,root,root)%{_sbindir}/%{name}
%config(noreplace)%{_sysconfdir}/%{name}/%{name}.env
%{_unitdir}/%{name}.service