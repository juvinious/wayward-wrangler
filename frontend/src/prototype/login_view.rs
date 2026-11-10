// Ephemeral local provider authentication; deliberately absent from journal/drafts.
#[derive(Default)]
struct LoginView {
    providers:Vec<Value>,provider:usize,method:usize,stage:u8,
    prompt:Value,prompt_id:String,input:Composer,choice:usize,status:String,url:String,return_settings:bool,
}
impl App {
    fn open_login(&mut self,selected:Option<String>){
        let mut providers=self.provider_state()["discovery"]["providers"].as_array().cloned().unwrap_or_default();
        providers.sort_by_key(|p|p["id"].as_str().unwrap_or("").to_owned());
        let selected=selected.or_else(||self.chat().and_then(|c|c["profile"]["provider"].as_str()).map(str::to_owned));
        let provider=providers.iter().position(|p|p["id"].as_str()==selected.as_deref()).unwrap_or(0);
        self.login=Some(LoginView{providers,provider,return_settings:self.modal==Modal::Settings,..LoginView::default()});self.open(Modal::Login);
    }
    pub fn auth_update(&mut self,event:Value){
        let Some(login)=&mut self.login else{return;};
        match event["type"].as_str(){
            Some("auth.prompt")=>{login.prompt=event["prompt"].clone();login.prompt_id=event["id"].as_str().unwrap_or("").into();login.input=Composer::default();login.choice=0;},
            Some("auth.prompt_cancelled") if event["id"]==login.prompt_id=>{login.prompt=Value::Null;login.input=Composer::default();},
            Some("auth.event")=>{
                let e=&event["event"];login.status=e["message"].as_str().or(e["instructions"].as_str()).unwrap_or("Complete provider sign-in").into();
                if let Some(url)=e["url"].as_str().or(e["verificationUri"].as_str()){
                    let open=login.url!=url&&e["openBrowserOnConfirm"]!=false;login.url=url.into();
                    if let Some(code)=e["userCode"].as_str(){login.status=format!("Browser verification code: {code}");}
                    if open{self.intent(LiveIntent::AuthBrowser(url.into()));}
                }
            },
            Some("auth.completed")=>{
                login.stage=3;login.prompt=Value::Null;login.input=Composer::default();login.url.clear();
                login.status=match event["status"].as_str(){Some("authenticated")=>"Signed in. Credentials saved privately. Send again when ready; held messages will not retry automatically.",Some("cancelled")=>"Sign-in cancelled. Your draft and project work are unchanged.",_=>"Sign-in did not complete. Check provider/account setup and try again."}.into();
                self.notice=login.status.clone();
            },
            _=>{}
        }
        self.mark();
        self.modal_scroll=0;
    }
    fn login_key(&mut self,event:Event){
        let Some(login)=&mut self.login else{return;};
        if let Event::Key(k)=&event&&k.kind!=KeyEventKind::Release{
            if k.kind==KeyEventKind::Repeat&&k.code==KeyCode::Enter{return;}
            if matches!(k.code,KeyCode::PageUp|KeyCode::PageDown){self.modal_scroll=if k.code==KeyCode::PageUp{self.modal_scroll.saturating_sub(8)}else{self.modal_scroll.saturating_add(8).min(self.modal_scroll_limit)};self.mark();return;}
            if k.code==KeyCode::Esc||(k.modifiers.contains(KeyModifiers::CONTROL)&&k.code==KeyCode::Char('c')){
                if k.code==KeyCode::Esc&&login.stage==1{login.stage=0;self.mark();return;}
                let parent=if login.return_settings{Modal::Settings}else{Modal::None};
                if login.stage==2{self.intent(LiveIntent::AuthCancel);self.notice="Sign-in cancelled; draft and project work retained".into();}
                self.login=None;self.modal=parent;self.mark();return;
            }
            if k.modifiers.contains(KeyModifiers::CONTROL)&&k.code==KeyCode::Char('b'){
                let url=login.url.clone();if !url.is_empty(){self.intent(LiveIntent::AuthBrowser(url));}return;
            }
            if login.stage==0||login.stage==1{
                let len=if login.stage==0{login.providers.len()}else{Self::login_methods(login).len()};
                let index=if login.stage==0{&mut login.provider}else{&mut login.method};
                match k.code{
                    KeyCode::Down|KeyCode::Tab=>*index=(*index+1).min(len.saturating_sub(1)),
                    KeyCode::Up|KeyCode::BackTab=>*index=index.saturating_sub(1),
                    KeyCode::Enter if len>0=>{
                        if login.stage==0{login.stage=1;login.method=0;}
                        else{
                            let provider=login.providers[login.provider]["id"].as_str().unwrap_or("").to_owned();
                            let method=Self::login_methods(login)[login.method].as_str().unwrap_or("").to_owned();
                            login.stage=2;login.status="Starting provider sign-in… Escape cancels.".into();
                            self.intent(LiveIntent::ProviderLogin{provider,method});
                        }
                    },_=>{}
                }
            }else if login.stage==3{
                if k.code==KeyCode::Enter{let parent=if login.return_settings{Modal::Settings}else{Modal::None};self.login=None;self.modal=parent;}
            }else if !login.prompt.is_null(){
                let options=login.prompt["options"].as_array();
                match k.code{
                    KeyCode::Down|KeyCode::Tab if options.is_some()=>login.choice=(login.choice+1).min(options.unwrap().len().saturating_sub(1)),
                    KeyCode::Up|KeyCode::BackTab if options.is_some()=>login.choice=login.choice.saturating_sub(1),
                    KeyCode::Enter=>{
                        let value=if let Some(options)=options{options.get(login.choice).and_then(|o|o["id"].as_str()).unwrap_or("").to_owned()}else{let value=login.input.text();login.input=Composer::default();value};
                        if !value.is_empty(){let id=login.prompt_id.clone();login.prompt=Value::Null;self.intent(LiveIntent::AuthAnswer{id,value});}
                    },_ if options.is_none()=>{
                        let newline=k.code==KeyCode::F(2)||(k.modifiers.contains(KeyModifiers::CONTROL)&&k.code==KeyCode::Char('j'));
                        let room=match k.code{KeyCode::Char('y') if k.modifiers.contains(KeyModifiers::CONTROL)=>login.input.text().len()+login.input.area.yank_text().len()<=4096,KeyCode::Char(c) if !k.modifiers.contains(KeyModifiers::CONTROL)=>login.input.text().len()+c.len_utf8()<=4096,_=>true};
                        if !newline&&room{login.input.input(event.clone());}
                    },_=>{}
                }
            }
        }else if let Event::Paste(text)=event&&login.stage==2&&!login.prompt.is_null()&&login.prompt["type"]!="select"{
            let value=text.trim_end_matches(['\r','\n']);if !value.contains(['\r','\n'])&&login.input.text().len()+value.len()<=4096{login.input.insert_paste(value);}
        }
        self.mark();
    }
    fn login_methods(login:&LoginView)->Vec<Value>{
        login.providers.get(login.provider).and_then(|p|p.get("login_methods").or_else(||p.get("methods"))).and_then(Value::as_array).cloned().unwrap_or_default()
    }
    fn login_body(&self)->(String,String){
        let Some(login)=&self.login else{return ("Sign in".into(),"No sign-in is active.".into());};
        let provider=login.providers.get(login.provider).and_then(|p|p["id"].as_str()).unwrap_or("provider");
        let mut body="Provider/account authentication, shared by its models. Model choice and project authority stay unchanged.\n\n".to_owned();
        match login.stage{
            0=>{
                body.push_str("Choose provider (Up/Down, Enter):\n");
                if login.providers.is_empty(){body.push_str("Provider metadata unavailable. Refresh the catalog in F3 with R, then return to /login.");}
                let start=login.provider.saturating_sub(5);for(i,p)in login.providers.iter().enumerate().skip(start).take(11){body.push_str(&format!("{} {}\n",if i==login.provider{"›"}else{" "},p["id"].as_str().unwrap_or("")));}
            },
            1=>{
                body.push_str(&format!("{provider} · choose the SDK sign-in method:\n"));let methods=Self::login_methods(login);
                if methods.is_empty(){body.push_str("This provider uses environment/configuration credentials; no interactive login is advertised by its SDK. No sign-in was started.");}
                for(i,m)in methods.iter().enumerate(){let id=m.as_str().unwrap_or("");let label=login.providers[login.provider]["login_method_labels"][id].as_str().unwrap_or(id);body.push_str(&format!("{} {} ({id})\n",if i==login.method{"›"}else{" "},label));}
            },
            _=>{
                body.push_str(&format!("{provider}\n{}\n",login.status));
                if !login.prompt.is_null(){
                    body.push_str(&format!("\n{}\n",login.prompt["message"].as_str().unwrap_or("Provider input")));
                    if let Some(options)=login.prompt["options"].as_array(){for(i,o)in options.iter().enumerate().skip(login.choice.saturating_sub(5)).take(11){body.push_str(&format!("{} {}\n",if i==login.choice{"›"}else{" "},o["label"].as_str().unwrap_or("")));}}
                    else{let shown=if matches!(login.prompt["type"].as_str(),Some("secret"|"manual_code")){"•".repeat(login.input.text().chars().count().min(64))}else{login.input.text()};body.push_str(&format!("> {shown}▏\nEnter submits · secrets/codes are not saved in project history\n"));}
                }
                if !login.url.is_empty(){body.push_str(&format!("\nBrowser sign-in: {}\nCtrl-B opens it again; manual redirect/code entry is supported when offered.\n",login.url));}
            }
        }
        ("Sign in to provider".into(),body)
    }
}
