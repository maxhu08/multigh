#!/usr/bin/env python3
import json
import os
import sys

args = sys.argv[1:]
with open(os.environ['MGH_GH_LOG'], 'a') as f:
    f.write(json.dumps(args) + '\n')
p = os.environ['MGH_AUTH']
with open(p) as f:
    data = json.load(f)

if args == ['--version']:
    print('gh version 2.99.0')
    sys.exit(0)

if args[:3] == ['config', 'get', 'user']:
    if os.getenv('MGH_SELECTED_EXPECT_OUTPUT'):
        with open(os.environ['MGH_SELECTED_EXPECT_OUTPUT']) as f:
            if 'Autoswitch: checking identity' not in f.read():
                print('autoswitch feedback missing before GitHub query', file=sys.stderr)
                sys.exit(2)
    if os.getenv('MGH_SELECTED_FAIL'):
        print('selected query exploded', file=sys.stderr)
        sys.exit(2)
    if data.get('selected') is None:
        print('user not set', file=sys.stderr)
        sys.exit(1)
    print(data['selected'])
    sys.exit(0)

if args[:2] == ['auth', 'status']:
    if os.getenv('MGH_QUERY_FAIL'):
        print('account query exploded', file=sys.stderr)
        sys.exit(2)
    if '--json' in args:
        if os.getenv('MGH_QUERY_BAD_JSON'):
            print('{broken')
            sys.exit(0)
        print(json.dumps({
            'hosts': {} if os.getenv('MGH_HOST_MISSING') else {
                'github.com': data['accounts'],
            },
        }))
        sys.exit(0)
    if os.getenv('MGH_NATIVE_LAYOUT'):
        sys.stdout.write('\n\nfirst auth line\n\n  last auth line\n\n')
        sys.stderr.write('\n\nnative warning without final newline')
        sys.exit(0)
    color = (
        os.getenv('NO_COLOR') is None
        and os.getenv('CLICOLOR') != '0'
        and (
            sys.stdout.isatty()
            or os.getenv('CLICOLOR_FORCE', '0') != '0'
            or os.getenv('GH_FORCE_TTY')
        )
    )
    if os.getenv('MGH_NATIVE_STYLE') and color:
        print('\x1b[31mforced native color\x1b[0m')
        print('\x1b[32mnative stderr color\x1b[0m', file=sys.stderr)
    print('native auth stdout')
    print('native auth stderr', file=sys.stderr)
    sys.exit(1 if os.getenv('MGH_FULL_FAIL') else 0)

if args[:2] == ['auth', 'login']:
    if os.getenv('MGH_LOGIN_FAIL'):
        print('browser login failed', file=sys.stderr)
        sys.exit(1)
    login = os.getenv('MGH_LOGIN_ACCOUNT', 'wrong')
    for a in data['accounts']:
        a['active'] = False
    data['accounts'] = [a for a in data['accounts'] if a['login'].lower() != login.lower()]
    data['accounts'].append({'login': login, 'active': True, 'state': 'success'})
    data['selected'] = login
    with open(p, 'w') as f:
        json.dump(data, f)
    if os.getenv('MGH_EDIT_CONFIG'):
        with open(os.environ['MGH_EDIT_CONFIG'], 'w') as f:
            f.write('{"concurrent":true}\n')
    if os.getenv('MGH_DELETE_CONFIG'):
        os.unlink(os.environ['MGH_DELETE_CONFIG'])
    sys.exit(0)
if args[:2] == ['auth', 'switch']:
    if os.getenv('MGH_SELECTED_EXPECT_OUTPUT'):
        with open(os.environ['MGH_SELECTED_EXPECT_OUTPUT']) as f:
            if 'personal is not allowed, switching to work…' not in f.read():
                print('switch feedback missing before GitHub switch', file=sys.stderr)
                sys.exit(1)
    if os.getenv('MGH_SWITCH_FAIL'):
        print('switch exploded', file=sys.stderr)
        sys.exit(1)
    login = args[args.index('--user') + 1]
    found = False
    for a in data['accounts']:
        a['active'] = a['login'].lower() == login.lower()
        if a['active'] and a['state'] == 'success':
            found = True
    if not found:
        print('account not signed in', file=sys.stderr)
        sys.exit(1)
    data['selected'] = login
    with open(p, 'w') as f:
        json.dump(data, f)
    sys.exit(0)
print('unexpected fake gh invocation ' + repr(args), file=sys.stderr)
sys.exit(3)
